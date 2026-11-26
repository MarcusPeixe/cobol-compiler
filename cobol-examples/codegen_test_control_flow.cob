       IDENTIFICATION DIVISION.
       PROGRAM-ID. CODEGEN-FLOW.

       DATA DIVISION.
       WORKING-STORAGE SECTION.
       77 I          PIC 9(2) VALUE ZERO.
       77 TOTAL      PIC 9(4) VALUE ZERO.
       77 NEG        PIC S9(3) VALUE -5.
       77 NAME       PIC X(8) VALUE "COBOL".
       77 CODE-X     PIC 9 VALUE 2.

       PROCEDURE DIVISION.
       MAIN-PARA.
           PERFORM VARYING I FROM 1 BY 1 UNTIL I > 5
               ADD I TO TOTAL
           END-PERFORM.
           DISPLAY "TOTAL=" TOTAL.
           PERFORM SHOW 2 TIMES.
           COMPUTE TOTAL = (TOTAL + 5) * 2 - 1.
           DISPLAY "TOTAL=" TOTAL.
           DISPLAY "NEG=" NEG.
           EVALUATE CODE-X
               WHEN 1 DISPLAY "ONE"
               WHEN 2 DISPLAY "TWO"
               WHEN OTHER DISPLAY "OTHER"
           END-EVALUATE.
           DISPLAY NAME(1:3).
           IF TOTAL > 30 AND NOT NAME = "X"
               DISPLAY "BIG"
           ELSE
               DISPLAY "SMALL"
           END-IF.
           GO TO FINISH-PARA.
           DISPLAY "SKIPPED".
       SHOW.
           DISPLAY "SHOW " NAME.
       FINISH-PARA.
           STOP RUN.
