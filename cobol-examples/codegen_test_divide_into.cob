       IDENTIFICATION DIVISION.
       PROGRAM-ID. CODEGEN-DIVIDE-INTO.
       DATA DIVISION.
       WORKING-STORAGE SECTION.
       77 WS-A PIC 99.
       77 WS-B PIC 99.
       PROCEDURE DIVISION.
       MAIN-PARA.
           MOVE 2 TO WS-A.
           MOVE 8 TO WS-B.
           DIVIDE WS-A INTO WS-A WS-B.
           DISPLAY "RESULT=" WS-A "," WS-B.
           DIVIDE 0 INTO WS-B
               ON SIZE ERROR DISPLAY "SIZE ERROR".
           DISPLAY "AFTER=" WS-B.
           MOVE 8 TO WS-B.
           DIVIDE 2 INTO WS-B GIVING WS-A.
           DISPLAY "GIVING=" WS-A.
           STOP RUN.
