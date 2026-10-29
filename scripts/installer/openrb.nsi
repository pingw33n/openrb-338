; The Windows installer of OpenRB-338 (task-111). Built by scripts/package-installer.sh, which
; passes the defines below; docs/building.md ("The Windows installer") describes it.
;
;   VERSION   YYYY.MMDD.HHMM, the build time (scripts/version.sh)
;   ARCH      x64 or arm64 (names the file, the title and the registry entry; the binaries
;             are those of the target the script was given)
;   BINDIR    the folder with OpenRB-338.exe and orb.exe (dist/<target>/bin)
;   STAGEDIR  the folder with LICENSE.txt and README.txt (CRLF)
;   PATHPS1   scripts/installer/path.ps1
;   ICON      assets/app_icon.ico (the icon of the installer and the uninstaller)
;   OUTFILE   the installer to write
;
; Per-user install (no administrator rights, no UAC prompt): the files go to
; %LOCALAPPDATA%\Programs\OpenRB-338, the shortcuts to the user's Start menu and desktop, the
; uninstall entry to HKCU, and the optional PATH change to the user's PATH. Why: the program is
; two executables with no service, driver or shared file; an unsigned installer that also asks
; for administrator rights is a second warning for nothing; and a per-user PATH needs no rights
; either. /D=DIR installs somewhere else, /S runs silently.
;
; Silent options (the checkboxes, for /S): /STARTMENU=0|1 (default 1), /DESKTOP=0|1 (default 0),
; /PATH=0|1 (default 0). Example:  setup.exe /S /PATH=1 /D=C:\Tools\OpenRB-338

Unicode true
SetCompressor /SOLID lzma
RequestExecutionLevel user
ManifestDPIAware true

!ifndef VERSION | ARCH | BINDIR | STAGEDIR | PATHPS1 | ICON | OUTFILE
  !error "VERSION, ARCH, BINDIR, STAGEDIR, PATHPS1, ICON and OUTFILE must be defined (-D)"
!endif

!define NAME "OpenRB-338"
!define UNINST_KEY "Software\Microsoft\Windows\CurrentVersion\Uninstall\${NAME}"
!define APP_KEY "Software\${NAME}"

Name "${NAME}"
OutFile "${OUTFILE}"
InstallDir "$LOCALAPPDATA\Programs\${NAME}"
InstallDirRegKey HKCU "${APP_KEY}" "InstallDir"
BrandingText "${NAME} ${VERSION} (${ARCH})"

VIProductVersion "${VERSION}.0"
VIAddVersionKey "ProductName" "${NAME}"
VIAddVersionKey "FileDescription" "${NAME} installer (${ARCH})"
VIAddVersionKey "FileVersion" "${VERSION}"
VIAddVersionKey "ProductVersion" "${VERSION}"
VIAddVersionKey "LegalCopyright" "GPL-3.0-only"

!include "MUI2.nsh"
!include "FileFunc.nsh"
!include "LogicLib.nsh"
!include "Sections.nsh"
!include "WinMessages.nsh"
!insertmacro GetParameters
!insertmacro GetOptions
!insertmacro GetSize

!define MUI_ICON "${ICON}"
!define MUI_UNICON "${ICON}"
!define MUI_ABORTWARNING

!define MUI_WELCOMEPAGE_TEXT "This installs ${NAME} ${VERSION} (${ARCH}), a reimplementation of the audio engine of Propellerhead ReBirth RB-338 2.01, and its command line tool orb.$\r$\n$\r$\nIt needs no file of the original program: the standard drum kit is built in.$\r$\n$\r$\nThe installer is not signed, which is why Windows asks whether to run it. It needs no administrator rights."
!insertmacro MUI_PAGE_WELCOME
!define MUI_LICENSEPAGE_TEXT_TOP "${NAME} is free software under the GNU General Public License, version 3 only."
!insertmacro MUI_PAGE_LICENSE "${STAGEDIR}\LICENSE.txt"
!insertmacro MUI_PAGE_COMPONENTS
!insertmacro MUI_PAGE_DIRECTORY
!insertmacro MUI_PAGE_INSTFILES
!define MUI_FINISHPAGE_RUN "$INSTDIR\${NAME}.exe"
!define MUI_FINISHPAGE_RUN_TEXT "Start ${NAME}"
!define MUI_FINISHPAGE_SHOWREADME "$INSTDIR\README.txt"
!define MUI_FINISHPAGE_SHOWREADME_TEXT "Show the readme"
!insertmacro MUI_PAGE_FINISH

!insertmacro MUI_UNPAGE_CONFIRM
!insertmacro MUI_UNPAGE_INSTFILES
!insertmacro MUI_UNPAGE_FINISH

!insertmacro MUI_LANGUAGE "English"

; ---------------------------------------------------------------------------------------------

; A running OpenRB-338.exe or orb.exe cannot be overwritten or deleted. Opening the file for
; writing fails while a program runs from it (a sharing violation), so that is the check; it
; changes nothing. EnsureClosed asks the user to close the program (Retry); under /S, or on
; Cancel, it sets the exit code 5 and aborts before anything is changed.
!macro LockFunctions UN
  Function ${UN}CheckLocked
    StrCpy $0 0
    ${ForEach} $2 1 2 + 1
      ${If} $2 == 1
        StrCpy $3 "$INSTDIR\${NAME}.exe"
      ${Else}
        StrCpy $3 "$INSTDIR\orb.exe"
      ${EndIf}
      ${If} ${FileExists} $3
        ClearErrors
        FileOpen $4 $3 a
        ${If} ${Errors}
          StrCpy $0 1
        ${Else}
          FileClose $4
        ${EndIf}
      ${EndIf}
    ${Next}
  FunctionEnd

  Function ${UN}EnsureClosed
    retry:
    Call ${UN}CheckLocked
    ${If} $0 == 1
      MessageBox MB_RETRYCANCEL|MB_ICONEXCLAMATION "${NAME} or orb is running from $INSTDIR.$\r$\nClose it, then choose Retry." /SD IDCANCEL IDRETRY retry
      SetErrorLevel 5
      Abort "${NAME} or orb is running from $INSTDIR: close it and run this again (exit code 5)."
    ${EndIf}
  FunctionEnd
!macroend
!insertmacro LockFunctions ""
!insertmacro LockFunctions "un."

Section "${NAME} and orb" SecApp
  SectionIn RO
  Call EnsureClosed
  ; An install over an earlier one keeps its folder, unless /D= or the folder page chose
  ; another: then the PATH entry of the earlier folder (if there is one) would point nowhere,
  ; so it goes. The files of the earlier folder stay.
  ReadRegStr $2 HKCU "${APP_KEY}" "InstallDir"
  ${If} $2 != ""
  ${AndIf} $2 != "$INSTDIR"
    InitPluginsDir
    File "/oname=$PLUGINSDIR\path.ps1" "${PATHPS1}"
    nsExec::ExecToLog 'powershell.exe -NoProfile -NonInteractive -ExecutionPolicy Bypass -File "$PLUGINSDIR\path.ps1" remove "$2"'
    Pop $0
    SendMessage ${HWND_BROADCAST} ${WM_WININICHANGE} 0 "STR:Environment" /TIMEOUT=5000
  ${EndIf}
  SetOutPath "$INSTDIR"
  File "${BINDIR}\${NAME}.exe"
  File "${BINDIR}\orb.exe"
  File "${STAGEDIR}\LICENSE.txt"
  File "${STAGEDIR}\README.txt"
  WriteUninstaller "$INSTDIR\Uninstall.exe"

  WriteRegStr HKCU "${APP_KEY}" "InstallDir" "$INSTDIR"
  WriteRegStr HKCU "${APP_KEY}" "Version" "${VERSION}"

  ; Settings > Apps > Installed apps (Add or remove programs)
  WriteRegStr HKCU "${UNINST_KEY}" "DisplayName" "${NAME}"
  WriteRegStr HKCU "${UNINST_KEY}" "DisplayVersion" "${VERSION}"
  WriteRegStr HKCU "${UNINST_KEY}" "Publisher" "${NAME}"
  WriteRegStr HKCU "${UNINST_KEY}" "DisplayIcon" "$INSTDIR\${NAME}.exe"
  WriteRegStr HKCU "${UNINST_KEY}" "InstallLocation" "$INSTDIR"
  WriteRegStr HKCU "${UNINST_KEY}" "UninstallString" '"$INSTDIR\Uninstall.exe"'
  WriteRegStr HKCU "${UNINST_KEY}" "QuietUninstallString" '"$INSTDIR\Uninstall.exe" /S'
  WriteRegDWORD HKCU "${UNINST_KEY}" "NoModify" 1
  WriteRegDWORD HKCU "${UNINST_KEY}" "NoRepair" 1
  ${GetSize} "$INSTDIR" "/S=0K" $0 $1 $2
  WriteRegDWORD HKCU "${UNINST_KEY}" "EstimatedSize" $0
SectionEnd

Section "Start menu shortcut" SecStartMenu
  CreateShortcut "$SMPROGRAMS\${NAME}.lnk" "$INSTDIR\${NAME}.exe"
SectionEnd

Section /o "Desktop shortcut" SecDesktop
  CreateShortcut "$DESKTOP\${NAME}.lnk" "$INSTDIR\${NAME}.exe"
SectionEnd

Section /o "Add orb to the PATH of the current user" SecPath
  SetOutPath "$INSTDIR"
  File "/oname=$INSTDIR\path.ps1" "${PATHPS1}"
  nsExec::ExecToLog 'powershell.exe -NoProfile -NonInteractive -ExecutionPolicy Bypass -File "$INSTDIR\path.ps1" add "$INSTDIR"'
  Pop $0
  ${If} $0 == 0
    ; tell running programs (Explorer, so new terminals) that the environment changed
    SendMessage ${HWND_BROADCAST} ${WM_WININICHANGE} 0 "STR:Environment" /TIMEOUT=5000
  ${Else}
    DetailPrint "Could not change the PATH (exit code $0). Add $INSTDIR to it by hand."
    MessageBox MB_OK|MB_ICONEXCLAMATION "The PATH could not be changed (exit code $0).$\r$\nAdd $INSTDIR to the PATH by hand to run orb from any folder." /SD IDOK
  ${EndIf}
SectionEnd

!insertmacro MUI_FUNCTION_DESCRIPTION_BEGIN
  !insertmacro MUI_DESCRIPTION_TEXT ${SecApp} "The app ${NAME}.exe and the command line tool orb.exe."
  !insertmacro MUI_DESCRIPTION_TEXT ${SecStartMenu} "A shortcut to ${NAME} in the Start menu."
  !insertmacro MUI_DESCRIPTION_TEXT ${SecDesktop} "A shortcut to ${NAME} on the desktop."
  !insertmacro MUI_DESCRIPTION_TEXT ${SecPath} "Adds the install folder to the PATH of your user account, so orb works in any new terminal."
!insertmacro MUI_FUNCTION_DESCRIPTION_END

; /STARTMENU=, /DESKTOP= and /PATH= set the checkboxes (for /S too)
!macro OptionSection NAME SEC
  ClearErrors
  ${GetOptions} $0 "/${NAME}=" $1
  ${IfNot} ${Errors}
    ${If} $1 == "1"
      !insertmacro SelectSection ${SEC}
    ${Else}
      !insertmacro UnselectSection ${SEC}
    ${EndIf}
  ${EndIf}
!macroend

Function .onInit
  ; the Start menu and desktop of this user, not of all users
  SetShellVarContext current
  ${GetParameters} $0
  !insertmacro OptionSection "STARTMENU" ${SecStartMenu}
  !insertmacro OptionSection "DESKTOP" ${SecDesktop}
  !insertmacro OptionSection "PATH" ${SecPath}
FunctionEnd

; ---------------------------------------------------------------------------------------------

Function un.onInit
  SetShellVarContext current
FunctionEnd

Section "Uninstall"
  ; nothing is changed while the program runs (exit code 5 under /S)
  Call un.EnsureClosed
  ; the PATH entry goes first, while path.ps1 is still there (nothing happens without an entry)
  ${If} ${FileExists} "$INSTDIR\path.ps1"
    nsExec::ExecToLog 'powershell.exe -NoProfile -NonInteractive -ExecutionPolicy Bypass -File "$INSTDIR\path.ps1" remove "$INSTDIR"'
    Pop $0
    SendMessage ${HWND_BROADCAST} ${WM_WININICHANGE} 0 "STR:Environment" /TIMEOUT=5000
  ${EndIf}

  Delete "$SMPROGRAMS\${NAME}.lnk"
  Delete "$DESKTOP\${NAME}.lnk"
  Delete "$INSTDIR\${NAME}.exe"
  Delete "$INSTDIR\orb.exe"
  Delete "$INSTDIR\LICENSE.txt"
  Delete "$INSTDIR\README.txt"
  Delete "$INSTDIR\path.ps1"
  Delete "$INSTDIR\Uninstall.exe"
  ; not recursive: a folder the user put files in stays
  RMDir "$INSTDIR"
  ; a file that could not be deleted keeps the entry in Apps, so the uninstall can be repeated
  ${If} ${FileExists} "$INSTDIR\${NAME}.exe"
  ${OrIf} ${FileExists} "$INSTDIR\orb.exe"
    SetErrorLevel 6
    MessageBox MB_OK|MB_ICONEXCLAMATION "Some files in $INSTDIR could not be deleted. The uninstall entry stays: close what uses them and uninstall again." /SD IDOK
    Abort "Files could not be deleted (exit code 6)."
  ${EndIf}
  DeleteRegKey HKCU "${UNINST_KEY}"
  DeleteRegKey HKCU "${APP_KEY}"
SectionEnd
