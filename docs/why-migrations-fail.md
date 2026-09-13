# Why mainframe migrations fail

## And the mechanism that makes ours falsifiable

**Bylazora - bylazora.com**

---

Every documented mainframe migration failure has the same shape: the new system was judged by inspection, and the first machine-checked test happened on production, in front of customers. This paper maps the named failures to the mechanisms Bylazora ships, so the claim is concrete: each of these would have failed our gate before cutover, not after.

## 1. TSB, 2018: validated by inspection, fined by regulators

A UK bank's core migration locked 1.9 million customers out of their accounts for weeks, required over GBP 360 million of remediation, and ended in a GBP 48.65 million regulator fine. The post-mortems ([IBM's findings](https://www.computerweekly.com/news/252443551/IBM-early-findings-on-disastrous-TSB-core-banking-system-migration-released), [the Slaughter and May report](https://www.theguardian.com/business/2019/nov/19/tsb-it-meltdown-report-computer-failure-accounts), [the FCA fine](https://www.thisismoney.co.uk/money/markets/article-11557371/TSB-fined-nearly-49m-City-regulators-botched-upgrade.html)) agree on the root cause: the migrated system was tested for function, never proven for equivalence. Behaviour was checked by eye and by sampling.

**The Bylazora mechanism:** acceptance is mechanical, not inspected. Every migrated job must reproduce its legacy reference output byte for byte, across the full corpus, enforced on every run. A system with TSB's defect profile fails the validator in the test environment - where a failure costs an afternoon, not a regulator.

## 2. KCB, 2024: wrong balances are a data-fidelity failure

Kenya Commercial Bank customers [overdrew accounts by USD 7.7 million](https://techcabal.com/2024/11/05/kcb-customers-overdraw-accounts-by-7-7-million/) during a migration whose glitches produced incorrect balances. The failure class is numeric: somewhere between the legacy and the new system, money stopped being exact.

**The Bylazora mechanism:** money travels as integer cents end to end - no float drift, by construction - and the output contract is byte-level: field values, totals, ordering, formatting. The validator would have flagged the wrong-balance outputs the first time they were produced, with a diff naming the account and the cents.

## 3. Suncorp: the multi-year write-off

Suncorp's core system replacement was [finally junked after years, leaving a AUD 90 million crater](https://www.itnews.com.au/news/suncorps-oracle-core-finally-junked-leaves-90m-crater-547982). Big-bang platform replacement compounds risk: every job must work before any job ships.

**The Bylazora mechanism:** migration is per job, not per platform. Each function carries its own byte-exact proof and its own cutover, so a failing function is found and bounded early. Nothing about the approach requires a big-bang date to be survived.

## 4. Westpac: when the brake is the strategy

Westpac [slammed the brakes on a core system change](http://www2.computerworld.co.nz/article/print/496763/westpac_slams_brakes_core_system_change/) rather than ride it into production - the rational response to a migration whose risk cannot be measured. The brake is expensive, and it is what every team does when the only verification tool is judgement.

**The Bylazora mechanism:** risk becomes measurable. Dual-run shadowing reconciles legacy and target outputs continuously, and every function is reversible until its decommission order is signed. Teams get evidence to decide with, instead of a brake to pull.

## 5. The industry odds, and the disciplines that beat them

Gartner's much-cited forecast - 70% of mainframe-exit efforts failing or being abandoned ([summary](https://www.zengines.ai/resource-collection/mainframe-modernization-gartner-70-percent-fail)) - is the aggregate of the cases above. The known-good disciplines that reduce the odds are documented independently: [dual-run risk control](https://plavno.io/company/insights/google-cloud-mainframe-migration-dual-run) and [semantic equivalence testing](https://dev.to/banuap/zero-variance-proving-cobol-to-java-semantic-equivalence-with-a-live-mainframe-emulator-on-aws-1ani). Bylazora productises both, and adds the part the industry does not publish: the byte-exact validator as the standing acceptance criterion, plus the correctness contract (float drift, look-ahead leakage, and non-idempotence are refused, not cautioned against).

## 6. The honest other side: staying is also failing

The failure case on the stay side is measured too: legacy systems cost the US [at least USD 40 billion during Covid](https://www.ft.com/content/7610a755-27e3-4792-a337-27d9b301000b), and [COBOL-era unemployment systems delayed benefits](https://www.atlantafed.org/research-and-data/publications/working-papers/2025/10/16/14-coboling-together-ui-benefits-how-delays-in-fiscal-stabilizers-affect-aggregate-consumption) for the people who needed them most. Inaction has a price; the Bylazora answer to that price is the profiling step: measure the estate, then choose the target state on evidence.

## The falsifiable claim

Every failure above was discovered by customers or regulators. Ours would be discovered by a test run, because the acceptance criterion is a comparison of bytes, not a comparison of confidence. That is the entire argument: **proof, not percentages - and the proof is falsifiable on your hardware, before cutover.**

---

*Sources are linked inline and cited in the Evidence Annex. Figures are the named cases' published numbers, not Bylazora estimates.*
