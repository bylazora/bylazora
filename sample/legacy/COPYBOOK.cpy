      *> TRANSACTION RECORD (DB2 UNLOAD, CSV REPRESENTATION)
       01  TXN-REC.
           05  TXN-ACCOUNT      PIC 9(7).
           05  TXN-AMOUNT       PIC 9(7).
           05  TXN-TYPE         PIC X.
      *> ACCOUNT BALANCE RECORD
       01  BAL-REC.
           05  BAL-ACCOUNT      PIC 9(7).
           05  BAL-AMOUNT       PIC S9(12).
