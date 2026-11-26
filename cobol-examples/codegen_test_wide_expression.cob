       IDENTIFICATION DIVISION.
       PROGRAM-ID. WIDE-EXPRESSION.
       DATA DIVISION.
       WORKING-STORAGE SECTION.
       77 WS-A PIC S9(12)V99 VALUE -1234567890.12.
       77 WS-RESULT PIC S9(12)V99.
       PROCEDURE DIVISION.
       MAIN-PARA.
           COMPUTE WS-RESULT = (WS-A + 5) * (2 - 0.5).
           DISPLAY "PRODUCT=" WS-RESULT.
           COMPUTE WS-RESULT = (1 / 3) * 3.
           DISPLAY "EXACT=" WS-RESULT.
           COMPUTE WS-RESULT = -(1 / 3 + 1 / 6) * -2.
           DISPLAY "FRACTIONS=" WS-RESULT.
           COMPUTE WS-RESULT = ((0 - 1) / (0 - 3)) * 3.
           DISPLAY "SIGNS=" WS-RESULT.
           COMPUTE WS-RESULT = (WS-A * -2) / (4 + 1).
           DISPLAY "QUOTIENT=" WS-RESULT.
           COMPUTE WS-RESULT = (WS-A + 1) / (0 - 2).
           DISPLAY "NEGATIVE-DIVISOR=" WS-RESULT.
           COMPUTE WS-A = (WS-A + 0.12) * -1.
           DISPLAY "ALIASED=" WS-A.
           COMPUTE WS-RESULT = 1 / (1 / (3 - 3)).
           DISPLAY "UNEXPECTED".
           STOP RUN.
