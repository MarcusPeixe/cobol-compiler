       IDENTIFICATION DIVISION.
       PROGRAM-ID. EVALS.
       DATA DIVISION.
       WORKING-STORAGE SECTION.
       77 WS-N PIC 99.
       PROCEDURE DIVISION.
       MAIN-PARA.
           EVALUATE WS-N
               WHEN 1
               WHEN 2
                   DISPLAY "ONE OR TWO"
               WHEN 3 THRU 5
                   DISPLAY "RANGE"
               WHEN OTHER
                   DISPLAY "OTHER"
           END-EVALUATE.
           EVALUATE TRUE
               WHEN WS-N < 10
                   DISPLAY "SMALL"
               WHEN WS-N >= 10 AND WS-N < 50
                   DISPLAY "MEDIUM"
               WHEN OTHER
                   DISPLAY "LARGE"
           END-EVALUATE.
           EVALUATE WS-N
               WHEN ANY
                   DISPLAY "ANY"
           END-EVALUATE.
           STOP RUN.
