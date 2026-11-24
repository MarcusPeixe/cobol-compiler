       IDENTIFICATION DIVISION.
           PROGRAM-ID. MULTIPLE-ERRORS-TEST.

       DATA DIVISION.
           WORKING-STORAGE SECTION.
           01 NAME PIC A(10).
           01 NAME PIC X(20).
           01 COUNT PIC 9(5).

       PROCEDURE DIVISION.
           MAIN.
               PERFORM UNDEFINED-PARAGRAPH.
               MOVE NAME TO COUNT.
               DISPLAY UNDEFINED-VAR.
               STOP RUN.
