       IDENTIFICATION DIVISION.
       PROGRAM-ID. INSPECT-MULTIPLE.
       DATA DIVISION.
       WORKING-STORAGE SECTION.
       77 WS-TEXT PIC X(6).
       PROCEDURE DIVISION.
       MAIN-PARA.
           MOVE "ABBAAA" TO WS-TEXT.
           INSPECT WS-TEXT REPLACING
               ALL "B" BY "Z"
               ALL "AB" BY "XY"
               FIRST "A" BY "Q".
           DISPLAY "PRIORITY=" WS-TEXT.
           MOVE "ABBAAA" TO WS-TEXT.
           INSPECT WS-TEXT REPLACING
               ALL "A" BY "B"
               ALL "B" BY "C".
           DISPLAY "ONCE=" WS-TEXT.
           MOVE "AAABAA" TO WS-TEXT.
           INSPECT WS-TEXT REPLACING
               FIRST "A" BY "Q"
               LEADING "A" BY "Z" AFTER INITIAL "B".
           DISPLAY "REGION=" WS-TEXT.
           MOVE "ABBAAA" TO WS-TEXT.
           INSPECT WS-TEXT REPLACING
               FIRST WS-TEXT BY "123456"
               CHARACTERS BY "X".
           DISPLAY "SNAPSHOT=" WS-TEXT.
           STOP RUN.
