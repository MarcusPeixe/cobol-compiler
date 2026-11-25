       IDENTIFICATION DIVISION.
       PROGRAM-ID. PERFORMS.
       DATA DIVISION.
       WORKING-STORAGE SECTION.
       77 WS-I PIC 99.
       77 WS-N PIC 99.
       PROCEDURE DIVISION.
       MAIN-PARA.
           PERFORM PARA-A.
           PERFORM PARA-A 3 TIMES.
           PERFORM PARA-A UNTIL WS-I > 5.
           PERFORM PARA-A THRU PARA-B.
           PERFORM PARA-A THROUGH PARA-B.
           PERFORM PARA-A VARYING WS-I FROM 1 BY 1 UNTIL WS-I > 3.
           PERFORM 2 TIMES
               DISPLAY "X"
           END-PERFORM.
           PERFORM UNTIL WS-N > 3
               ADD 1 TO WS-N
           END-PERFORM.
           PERFORM VARYING WS-I FROM 1 BY 2 UNTIL WS-I > 9
               DISPLAY WS-I
           END-PERFORM.
           PERFORM WITH TEST BEFORE UNTIL WS-N > 5
               ADD 1 TO WS-N
           END-PERFORM.
           STOP RUN.
       PARA-A.
           DISPLAY "A".
       PARA-B.
           DISPLAY "B".
