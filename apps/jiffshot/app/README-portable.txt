Jiffshot 0.1.0 (portable)
=========================

Screen capture and image editor for Windows 10 and 11.

Getting started
- Unzip this folder anywhere and run jiffshot.exe. Nothing is installed.
- Jiffshot lives in the notification area (the tray icon). Its menu
  captures, records video, opens the editor and has Preferences.
- Print Screen captures (Ctrl+Alt+Print Screen while another app, such
  as another capture tool or the Snipping Tool, holds Print Screen). Click
  a window, drag an area, or press F1 while selecting for all keys.
- Closing the editor keeps Jiffshot in the tray; the tray's Exit quits.

Where things go
- Captures: Pictures\Jiffshot (Preferences > Library to change it).
- Settings: %APPDATA%\Jiffshot\jiffshot.json.
- To remove Jiffshot, quit it and delete this folder (and those two, if
  you like). If you turned on "Run when Windows starts", turn it off in
  Preferences first.

Notes
- Windows may warn that the app is from an unknown publisher: the build
  is not code-signed. Choose More info > Run anyway.
- Video recording uses Windows Media Foundation, which Windows "N"
  editions leave out (Media Feature Pack from Microsoft adds it back).
