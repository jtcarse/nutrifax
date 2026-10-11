# Use Tauri for Windows and Android

Build the initial app with Tauri 2 and a shared interface for Windows and Android. The app needs native file selection, image export, and account sign-in while remaining usable on phone-sized screens. A browser-only app would simplify distribution, but Tauri better matches the planned installed-app experience and lets the two target platforms share most of the UI and application logic. Keep platform-specific file and sign-in behavior behind small interfaces. iOS is outside the initial target.
