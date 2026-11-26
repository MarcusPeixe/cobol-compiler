       IDENTIFICATION DIVISION.
       PROGRAM-ID. CODEGEN-SEARCH-ALL.
       DATA DIVISION.
       WORKING-STORAGE SECTION.
       01 WS-TABLE.
          05 WS-ENTRY OCCURS 4 TIMES INDEXED BY WS-INDEX.
             10 WS-ITEM PIC X.
       PROCEDURE DIVISION.
       MAIN-PARA.
           MOVE "ABCD" TO WS-TABLE.
           SET WS-INDEX TO 4.
           SEARCH ALL WS-ENTRY
               AT END DISPLAY "NOT FOUND"
               WHEN WS-ITEM(WS-INDEX) = "B"
                   DISPLAY "FOUND=" WS-INDEX
           END-SEARCH.
           SEARCH ALL WS-ENTRY
               AT END DISPLAY "MISSING"
               WHEN WS-ITEM(WS-INDEX) = "Z"
                   DISPLAY "UNEXPECTED"
           END-SEARCH.
           STOP RUN.
