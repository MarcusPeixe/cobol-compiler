       IDENTIFICATION DIVISION.
           PROGRAM-ID. VALID-TEST.

       DATA DIVISION.
           WORKING-STORAGE SECTION.
           01 MESSAGE PIC X(20).
           01 NAME PIC A(10).
           01 COUNTER PIC 9(5).

       PROCEDURE DIVISION.
           MAIN.
               MOVE "Hello" TO MESSAGE.
               MOVE "World" TO NAME.
               MOVE 42 TO COUNTER.
               DISPLAY MESSAGE.
               STOP RUN.
