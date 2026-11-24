       IDENTIFICATION DIVISION.
           PROGRAM-ID. DUPLICATE-TEST.

       DATA DIVISION.
           WORKING-STORAGE SECTION.
           01 MESSAGE PIC X(20).
           01 MESSAGE PIC 9(5).

       PROCEDURE DIVISION.
           MAIN.
               MOVE "Hello" TO MESSAGE.
               STOP RUN.
