       IDENTIFICATION DIVISION.
       PROGRAM-ID. TABLES.
       DATA DIVISION.
       WORKING-STORAGE SECTION.
       77 WS-PLAIN PIC 9.
       77 WS-I PIC 9.
       01 WS-TABLE.
          05 WS-ROW OCCURS 3 TIMES INDEXED BY ROW-IDX.
             10 WS-COL PIC 9 OCCURS 2 TIMES.
             10 WS-NAME PIC X(4).
       PROCEDURE DIVISION.
       MAIN-PARA.
           MOVE 1 TO WS-PLAIN (1).
           MOVE 1 TO WS-COL (WS-I).
           MOVE 1 TO WS-COL (WS-I, WS-I).
           MOVE "AB" TO WS-NAME (ROW-IDX).
           SEARCH WS-PLAIN
               AT END DISPLAY "NONE"
               WHEN WS-PLAIN = 1 DISPLAY "ONE"
           END-SEARCH.
           STOP RUN.
