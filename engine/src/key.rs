// SPDX-License-Identifier: AGPL-3.0-or-later
// Licence keys: a licensee record, not a gate.
//
// Keys are Ed25519-signed tokens verified offline: the binary holds only the
// public key, so reversing the binary cannot forge keys. Nothing here gates a
// run: the engine is free and unlimited under AGPL-3.0-or-later, so a key is
// verified and reported, for the run record and the licensee's own audit, but
// never enforced. What the licence sells is the AGPL exception, support and the
// engagement - the parts that cannot be patched out (see
// docs/legal/licensing-model.md). No network, no telemetry, ever: resolution is
// a pure function of the key and the clock, so the engine runs air-gapped.
use ed25519_dalek::{Signature, Verifier, VerifyingKey};
#[cfg(test)]
use ed25519_dalek::{Signer, SigningKey};
use std::path::PathBuf;

/// The production signing key's public half. Not a secret: publishing it is
/// the point of an offline-verified licence. Used for real verification only
/// outside test builds; see verifying_key_bytes().
const PUBLIC_KEY_HEX: &str = "2f05021850c3322c875ed35d88ed6e6724bfdcaaca0c89ecfe5fb026de0b8e35";
const KEY_PREFIX: &str = "bylazora:pro:v2:";
const LICENCE_URL: &str = "https://bylazora.com/licence.html";

/// A throwaway seed for test builds only. It signs nothing outside this
/// crate's own tests and verifies against no production key; it exists so
/// the test suite can construct valid-looking keys without a real signed
/// licence ever sitting in the source tree.
#[cfg(test)]
const TEST_SEED: [u8; 32] = [0xABu8; 32];

/// The key bytes verification actually checks against: the production
/// public key normally, or the test-only key when compiled for tests. This
/// is the only thing #[cfg(test)] changes about verification; the rest of
/// License::parse is identical in both builds.
fn verifying_key_bytes() -> [u8; 32] {
    #[cfg(test)]
    {
        SigningKey::from_bytes(&TEST_SEED).verifying_key().to_bytes()
    }
    #[cfg(not(test))]
    {
        decode_hex(PUBLIC_KEY_HEX).expect("public key is hex").try_into().expect("32 bytes")
    }
}

fn decode_hex(s: &str) -> Option<Vec<u8>> {
    if s.len() % 2 != 0 { return None; }
    (0..s.len()).step_by(2).map(|i| u8::from_str_radix(&s[i..i + 2], 16).ok()).collect()
}

/// Days since 1970-01-01 for a civil date (Howard Hinnant's algorithm).
pub fn days_from_civil(y: i64, m: u32, d: u32) -> i64 {
    let y = if m <= 2 { y - 1 } else { y };
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = y - era * 400;
    let doy = (153 * (if m > 2 { m - 3 } else { m + 9 }) as i64 + 2) / 5 + d as i64 - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146097 + doe - 719468
}

fn now_days() -> u64 {
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0);
    secs / 86400
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Tier { Pro }

#[derive(Debug, Clone, PartialEq)]
pub struct License {
    pub org: String,
    pub tier: Tier,
    pub expiry: String,
    pub expiry_days: u64,
}

impl License {
    /// Parse and verify a key of the form bylazora-v2-<org>-<expiryYYYYMMDD>-<sig>.
    /// <org> may itself contain hyphens (keygen and the portal both allow
    /// them), so this can't split left-to-right into a fixed field count;
    /// it strips the fixed prefix, then peels sig and expiry off the RIGHT
    /// (both fixed-width), leaving whatever remains, hyphens included, as
    /// the org.
    pub fn parse(key: &str) -> Result<License, String> {
        let rest = key.strip_prefix("bylazora-v2-").ok_or("unknown key format")?;
        let mut fields = rest.rsplitn(3, '-');
        let sig_str = fields.next().ok_or("unknown key format")?;
        let expiry = fields.next().ok_or("unknown key format")?;
        let org = fields.next().ok_or("unknown key format")?;

        if org.is_empty() || org.len() > 32
            || !org.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
        {
            return Err("invalid organisation in key".into());
        }
        if expiry.len() != 8 || !expiry.chars().all(|c| c.is_ascii_digit()) {
            return Err("invalid expiry in key".into());
        }
        let y: i64 = expiry[0..4].parse().map_err(|_| "invalid expiry in key")?;
        let m: u32 = expiry[4..6].parse().map_err(|_| "invalid expiry in key")?;
        let d: u32 = expiry[6..8].parse().map_err(|_| "invalid expiry in key")?;
        if m < 1 || m > 12 || d < 1 || d > 31 {
            return Err("invalid expiry in key".into());
        }
        let expiry_days = days_from_civil(y, m, d) as u64;
        let sig = decode_hex(sig_str).ok_or("invalid key signature")?;
        if sig.len() != 64 { return Err("invalid key signature".into()); }
        let vk = VerifyingKey::from_bytes(&verifying_key_bytes()).expect("valid public key");
        let signature = Signature::from_slice(&sig).map_err(|_| "invalid key signature".to_string())?;
        let msg = format!("{KEY_PREFIX}{org}:{expiry}");
        vk.verify(msg.as_bytes(), &signature).map_err(|_| "invalid key signature".to_string())?;
        Ok(License { org: org.to_string(), tier: Tier::Pro, expiry: expiry.to_string(), expiry_days })
    }

    /// The key is valid only up to and including its expiry day.
    pub fn check_expiry(&self, now: u64) -> Result<(), String> {
        if now > self.expiry_days {
            return Err(format!("licence expired {}: renew at {LICENCE_URL}", self.expiry));
        }
        Ok(())
    }
}

/// What resolving a licence produced. Reported, never enforced.
#[derive(Debug, PartialEq)]
pub enum LicenceOutcome {
    /// No key installed. An unlicensed run, which is entirely normal and has no
    /// limit of any kind.
    None,
    /// A valid, current key.
    Valid(License),
    /// A key was presented and its signature checks out, but it is past its
    /// expiry. Reported so the run record is honest about it.
    Expired(String),
    /// A key was presented and failed to parse or verify.
    Invalid(String),
}

/// Pure resolution: key and "today" in days since epoch, no I/O, no printing.
/// Split out from announce_licence so the behaviour is directly testable.
pub fn resolve(key: Option<&str>, now_days: u64) -> LicenceOutcome {
    let Some(k) = key else { return LicenceOutcome::None };
    match License::parse(k) {
        Err(e) => LicenceOutcome::Invalid(e),
        Ok(lic) => match lic.check_expiry(now_days) {
            Ok(()) => LicenceOutcome::Valid(lic),
            Err(e) => LicenceOutcome::Expired(e),
        },
    }
}

/// Report the licence for a run, and return what it resolved to.
///
/// There is no scale gate and no expiry gate. The engine is free and unlimited
/// under AGPL-3.0-or-later, so nothing about a licence stops a run: a key is
/// verified and announced so the run record can name the licensee, and a key
/// that is expired or malformed is announced too, and the run proceeds either
/// way. Anything else would make installing a key worse than not installing
/// one, which is the opposite of honest.
pub fn announce_licence() -> LicenceOutcome {
    let outcome = resolve(load_key().as_deref(), now_days());
    match &outcome {
        LicenceOutcome::None => {}
        LicenceOutcome::Valid(lic) => eprintln!(
            "licence: bylazora v2 {:?}, licensed to {} until {}",
            lic.tier, lic.org, lic.expiry
        ),
        LicenceOutcome::Expired(e) => eprintln!(
            "licence: {e}. The run is unaffected: this build is AGPL. Renew at {LICENCE_URL}"
        ),
        LicenceOutcome::Invalid(e) => eprintln!(
            "licence: key rejected: {e}. The run is unaffected: this build is AGPL"
        ),
    }
    outcome
}

/// Where the licence key lives: BYLAZORA_KEY_FILE, else the XDG config dir.
pub fn licence_file_path() -> PathBuf {
    if let Ok(p) = std::env::var("BYLAZORA_KEY_FILE") { return PathBuf::from(p); }
    if let Ok(x) = std::env::var("XDG_CONFIG_HOME") {
        return PathBuf::from(x).join("bylazora").join("licence.key");
    }
    if let Ok(h) = std::env::var("HOME") {
        return PathBuf::from(h).join(".config").join("bylazora").join("licence.key");
    }
    PathBuf::from("licence.key")
}

/// Resolve the key: BYLAZORA_KEY env wins, then the key file.
pub fn load_key() -> Option<String> {
    if let Ok(k) = std::env::var("BYLAZORA_KEY") {
        let t = k.trim().to_string();
        if !t.is_empty() { return Some(t); }
    }
    std::fs::read_to_string(licence_file_path())
        .ok().map(|s| s.trim().to_string()).filter(|s| !s.is_empty())
}

/// Install a key to a file, accepting only keys that are valid and unexpired.
pub fn install_key(key: &str, file: Option<&PathBuf>) -> Result<PathBuf, String> {
    let lic = License::parse(key)?;
    lic.check_expiry(now_days())?;
    let p = file.cloned().unwrap_or_else(licence_file_path);
    if let Some(dir) = p.parent() {
        std::fs::create_dir_all(dir).map_err(|e| format!("cannot create {}: {e}", dir.display()))?;
    }
    std::fs::write(&p, key.trim()).map_err(|e| format!("cannot write {}: {e}", p.display()))?;
    Ok(p)
}

pub fn cmd_set(key: &str, file: Option<&PathBuf>) -> Result<String, String> {
    let p = install_key(key, file)?;
    Ok(format!("installed licence key at {}", p.display()))
}

pub fn cmd_show() -> Result<String, String> {
    let k = load_key().ok_or_else(|| {
        "no licence key installed (set BYLAZORA_KEY or run: bylazora-core licence set <key>)".to_string()
    })?;
    let lic = License::parse(&k)?;
    Ok(format!("Pro tier, licensed to {} until {}", lic.org, lic.expiry))
}

pub fn cmd_check() -> Result<(), String> {
    let k = load_key().ok_or_else(|| "no licence key installed".to_string())?;
    let lic = License::parse(&k)?;
    lic.check_expiry(now_days())
}

pub fn cmd_clear(file: Option<&PathBuf>) -> Result<String, String> {
    let p = file.cloned().unwrap_or_else(licence_file_path);
    match std::fs::remove_file(&p) {
        Ok(()) => Ok(format!("removed {}", p.display())),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Err("no licence file to remove".into()),
        Err(e) => Err(e.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // days since epoch for 2027-12-31 (day 0 = 1970-01-01)
    const EXPIRY_DAYS: u64 = 21183;

    /// Sign a message with the test-only seed and return the 128-char hex
    /// signature. Verifies only against verifying_key_bytes() under
    /// #[cfg(test)]; a key built from this can never pass in a release
    /// build, and no production seed is used or needed to produce it.
    fn sign_test(msg: &str) -> String {
        let sk = SigningKey::from_bytes(&TEST_SEED);
        let sig = sk.sign(msg.as_bytes());
        sig.to_bytes().iter().map(|b| format!("{b:02x}")).collect()
    }

    fn test_key(org: &str, expiry: &str) -> String {
        let msg = format!("{KEY_PREFIX}{org}:{expiry}");
        format!("bylazora-v2-{org}-{expiry}-{}", sign_test(&msg))
    }

    fn acme_key() -> String {
        test_key("acme", "20271231")
    }

    #[test]
    fn production_public_key_hex_is_well_formed() {
        // A public value, not a secret: this only checks the production
        // constant decodes to a 32-byte Ed25519 public key.
        assert_eq!(decode_hex(PUBLIC_KEY_HEX).map(|v| v.len()), Some(32));
    }

    #[test]
    fn civil_to_days_matches_known_date() {
        assert_eq!(days_from_civil(2027, 12, 31), EXPIRY_DAYS as i64);
    }

    #[test]
    fn hyphenated_org_parses_correctly() {
        // keygen and the portal both allow '-' in an org slug (e.g. an
        // auto-generated "acme-corp" from "Acme Corp Pty Ltd"); parsing
        // must not mistake a hyphen inside the org for a field separator.
        let key = test_key("acme-corp", "20271231");
        let lic = License::parse(&key).expect("hyphenated org should parse");
        assert_eq!(lic.org, "acme-corp");
    }

    #[test]
    fn known_good_vector_parses() {
        let lic = License::parse(&acme_key()).expect("valid key");
        assert_eq!(lic.org, "acme");
        assert_eq!(lic.tier, Tier::Pro);
        assert_eq!(lic.expiry_days, EXPIRY_DAYS);
    }

    #[test]
    fn tampered_signature_fails() {
        let k = acme_key();
        let bad = k[..k.len() - 1].to_string() + if k.ends_with('0') { "1" } else { "0" };
        assert!(License::parse(&bad).is_err());
    }

    #[test]
    fn wrong_org_key_fails() {
        // Valid signature, but for "acme"; presenting it under "other"
        // must fail because the org is part of the signed message.
        let bad = format!("bylazora-v2-other-20271231-{}", sign_test(&format!("{KEY_PREFIX}acme:20271231")));
        assert!(License::parse(&bad).is_err());
    }

    #[test]
    fn the_engine_has_no_scale_gate() {
        // resolve() takes no row count at all, and that is the point: there is
        // nothing left to gate. A billion-row job with no key is an ordinary
        // unlicensed run, not an error. If a rows parameter ever reappears in
        // this signature, this test stops compiling and forces the argument.
        assert_eq!(resolve(None, EXPIRY_DAYS), LicenceOutcome::None);
    }

    #[test]
    fn no_key_is_a_normal_unlicensed_run() {
        assert_eq!(resolve(None, EXPIRY_DAYS), LicenceOutcome::None);
    }

    #[test]
    fn a_valid_key_resolves_as_licensed() {
        match resolve(Some(&acme_key()), EXPIRY_DAYS - 365) {
            LicenceOutcome::Valid(lic) => {
                assert_eq!(lic.org, "acme");
                assert_eq!(lic.tier, Tier::Pro);
            }
            other => panic!("expected a valid licence, got {other:?}"),
        }
    }

    #[test]
    fn expiry_boundary_last_day_still_resolves_as_licensed() {
        assert!(matches!(resolve(Some(&acme_key()), EXPIRY_DAYS), LicenceOutcome::Valid(_)));
    }

    #[test]
    fn an_expired_key_is_reported_but_does_not_block() {
        // Expiry ends the commercial grant. It does not stop the binary, because
        // the AGPL grant does not expire. Blocking here would make installing a
        // key worse than installing nothing, which is the opposite of honest.
        match resolve(Some(&acme_key()), EXPIRY_DAYS + 1) {
            LicenceOutcome::Expired(msg) => {
                assert!(msg.contains("expired"), "should say expired: {msg}")
            }
            other => panic!("expected Expired, got {other:?}"),
        }
    }

    #[test]
    fn an_invalid_key_is_reported_but_does_not_block() {
        let bad = "bylazora-v2-acme-20271231-deadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeef";
        match resolve(Some(bad), EXPIRY_DAYS - 365) {
            LicenceOutcome::Invalid(msg) => assert!(!msg.is_empty(), "should explain the failure"),
            other => panic!("expected Invalid, got {other:?}"),
        }
    }
}
