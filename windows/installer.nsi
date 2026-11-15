; Liyamu installer (NSIS 3, Unicode). Built by windows/build.sh, which passes VERSION and STAGE (the folder
; with x64\, x86\, arm64\, fonts\, liyamu-settings*.exe). Silent: LiyamuSetup.exe /S; uninstall: /S too.
Unicode true
ManifestDPIAware true
SetCompressor /SOLID lzma
RequestExecutionLevel admin

!include MUI2.nsh
!include LogicLib.nsh
!include x64.nsh

!ifndef VERSION
  !error "VERSION not defined"
!endif
!ifndef STAGE
  !error "STAGE not defined"
!endif

!define NAME "Liyamu"
!define PUBLISHER "Path Nirvana Foundation"
!define UNINSTALL_KEY "Software\Microsoft\Windows\CurrentVersion\Uninstall\Liyamu"

Name "${NAME}"
Caption "Liyamu ලියමු ${VERSION}"
BrandingText "Liyamu ${VERSION}: free and open Sinhala keyboard"
OutFile "${OUTFILE}"
InstallDir "$PROGRAMFILES64\Liyamu"
VIProductVersion "${VERSION}.0"
VIAddVersionKey "ProductName" "Liyamu"
VIAddVersionKey "CompanyName" "${PUBLISHER}"
VIAddVersionKey "FileDescription" "Liyamu Sinhala keyboard installer"
VIAddVersionKey "FileVersion" "${VERSION}"
VIAddVersionKey "ProductVersion" "${VERSION}"
VIAddVersionKey "LegalCopyright" "${PUBLISHER}, GPL-3.0-or-later"

!define MUI_ICON "res\liyamu.ico"
!define MUI_UNICON "res\liyamu.ico"
!define MUI_FINISHPAGE_RUN "$INSTDIR\liyamu-settings.exe"
!define MUI_FINISHPAGE_RUN_PARAMETERS "--help"
!define MUI_FINISHPAGE_RUN_TEXT "Open Liyamu settings and help"

!insertmacro MUI_PAGE_LICENSE "..\LICENSE"
!insertmacro MUI_PAGE_INSTFILES
!insertmacro MUI_PAGE_FINISH
!insertmacro MUI_UNPAGE_CONFIRM
!insertmacro MUI_UNPAGE_INSTFILES
!insertmacro MUI_LANGUAGE "English"

; A DLL that apps still have loaded cannot be overwritten or deleted, but it can be renamed: move it aside
; and delete it at the next restart.
!macro MoveAside path
  Delete "${path}"
  ${If} ${FileExists} "${path}"
    Delete "${path}.old"
    Rename "${path}" "${path}.old"
    Delete /REBOOTOK "${path}.old"
  ${EndIf}
!macroend

Function .onInit
  ${IfNot} ${RunningX64}
    MessageBox MB_ICONSTOP "Liyamu needs 64-bit Windows 10 or 11." /SD IDOK
    Abort
  ${EndIf}
  SetRegView 64
FunctionEnd

Function un.onInit
  SetRegView 64
FunctionEnd

Section "Liyamu"
  ${DisableX64FSRedirection}
  ; an earlier version: unregister it first
  ${If} ${FileExists} "$INSTDIR\x86\liyamu_ime.dll"
    ExecWait '"$WINDIR\SysWOW64\regsvr32.exe" /s /u "$INSTDIR\x86\liyamu_ime.dll"'
  ${EndIf}
  ${If} ${FileExists} "$INSTDIR\x64\liyamu_ime.dll"
    ExecWait '"$WINDIR\System32\regsvr32.exe" /s /u "$INSTDIR\x64\liyamu_ime.dll"'
  ${EndIf}
  ${If} ${FileExists} "$INSTDIR\arm64\liyamu_ime.dll"
    ExecWait '"$WINDIR\System32\regsvr32.exe" /s /u "$INSTDIR\arm64\liyamu_ime.dll"'
  ${EndIf}

  SetOutPath "$INSTDIR\x86"
  !insertmacro MoveAside "$INSTDIR\x86\liyamu_ime.dll"
  File "${STAGE}\x86\liyamu_ime.dll"
  ${If} ${IsNativeARM64}
    SetOutPath "$INSTDIR\arm64"
    !insertmacro MoveAside "$INSTDIR\arm64\liyamu_ime.dll"
    File "${STAGE}\arm64\liyamu_ime.dll"
    SetOutPath "$INSTDIR"
    !insertmacro MoveAside "$INSTDIR\liyamu-settings.exe"
    File /oname=liyamu-settings.exe "${STAGE}\liyamu-settings-arm64.exe"
  ${Else}
    SetOutPath "$INSTDIR\x64"
    !insertmacro MoveAside "$INSTDIR\x64\liyamu_ime.dll"
    File "${STAGE}\x64\liyamu_ime.dll"
    SetOutPath "$INSTDIR"
    !insertmacro MoveAside "$INSTDIR\liyamu-settings.exe"
    File /oname=liyamu-settings.exe "${STAGE}\liyamu-settings-x64.exe"
  ${EndIf}
  SetOutPath "$INSTDIR\fonts"
  File "${STAGE}\fonts\AbhayaUni-Regular.ttf"
  SetOutPath "$INSTDIR"
  File /oname=LICENSE.txt "..\LICENSE"

  ; 32-bit apps load the x86 DLL, 64-bit apps the x64 one (arm64 on ARM PCs)
  ExecWait '"$WINDIR\SysWOW64\regsvr32.exe" /s "$INSTDIR\x86\liyamu_ime.dll"' $0
  ${If} ${IsNativeARM64}
    ExecWait '"$WINDIR\System32\regsvr32.exe" /s "$INSTDIR\arm64\liyamu_ime.dll"' $1
  ${Else}
    ExecWait '"$WINDIR\System32\regsvr32.exe" /s "$INSTDIR\x64\liyamu_ime.dll"' $1
  ${EndIf}
  ${EnableX64FSRedirection}
  ${If} $0 != 0
  ${OrIf} $1 != 0
    MessageBox MB_ICONEXCLAMATION "The keyboard could not be registered with Windows ($0, $1)." /SD IDOK
  ${EndIf}

  ; add Liyamu to this user's Sinhala keyboards
  ExecWait '"$INSTDIR\liyamu-settings.exe" --add-keyboard'

  WriteUninstaller "$INSTDIR\uninstall.exe"
  CreateShortCut "$SMPROGRAMS\Liyamu.lnk" "$INSTDIR\liyamu-settings.exe" "" "$INSTDIR\liyamu-settings.exe" 0

  WriteRegStr HKLM "${UNINSTALL_KEY}" "DisplayName" "Liyamu Sinhala keyboard"
  WriteRegStr HKLM "${UNINSTALL_KEY}" "DisplayVersion" "${VERSION}"
  WriteRegStr HKLM "${UNINSTALL_KEY}" "Publisher" "${PUBLISHER}"
  WriteRegStr HKLM "${UNINSTALL_KEY}" "DisplayIcon" "$INSTDIR\liyamu-settings.exe"
  WriteRegStr HKLM "${UNINSTALL_KEY}" "InstallLocation" "$INSTDIR"
  WriteRegStr HKLM "${UNINSTALL_KEY}" "URLInfoAbout" "https://arutha.lk/liyamu"
  WriteRegStr HKLM "${UNINSTALL_KEY}" "UninstallString" '"$INSTDIR\uninstall.exe"'
  WriteRegStr HKLM "${UNINSTALL_KEY}" "QuietUninstallString" '"$INSTDIR\uninstall.exe" /S'
  WriteRegDWORD HKLM "${UNINSTALL_KEY}" "NoModify" 1
  WriteRegDWORD HKLM "${UNINSTALL_KEY}" "NoRepair" 1
  WriteRegDWORD HKLM "${UNINSTALL_KEY}" "EstimatedSize" 40000
SectionEnd

Section "Uninstall"
  ExecWait '"$INSTDIR\liyamu-settings.exe" --remove-keyboard'
  ${DisableX64FSRedirection}
  ExecWait '"$WINDIR\SysWOW64\regsvr32.exe" /s /u "$INSTDIR\x86\liyamu_ime.dll"'
  ${If} ${FileExists} "$INSTDIR\x64\liyamu_ime.dll"
    ExecWait '"$WINDIR\System32\regsvr32.exe" /s /u "$INSTDIR\x64\liyamu_ime.dll"'
  ${EndIf}
  ${If} ${FileExists} "$INSTDIR\arm64\liyamu_ime.dll"
    ExecWait '"$WINDIR\System32\regsvr32.exe" /s /u "$INSTDIR\arm64\liyamu_ime.dll"'
  ${EndIf}
  ${EnableX64FSRedirection}
  Delete /REBOOTOK "$INSTDIR\x86\liyamu_ime.dll"
  Delete /REBOOTOK "$INSTDIR\x64\liyamu_ime.dll"
  Delete /REBOOTOK "$INSTDIR\arm64\liyamu_ime.dll"
  Delete "$INSTDIR\liyamu-settings.exe"
  Delete "$INSTDIR\fonts\AbhayaUni-Regular.ttf"
  Delete "$INSTDIR\LICENSE.txt"
  Delete "$INSTDIR\uninstall.exe"
  RMDir /REBOOTOK "$INSTDIR\x86"
  RMDir /REBOOTOK "$INSTDIR\x64"
  RMDir /REBOOTOK "$INSTDIR\arm64"
  RMDir "$INSTDIR\fonts"
  RMDir /REBOOTOK "$INSTDIR"
  Delete "$SMPROGRAMS\Liyamu.lnk"
  DeleteRegKey HKLM "${UNINSTALL_KEY}"
  ; the user's settings, shortcuts and learned words stay unless they delete them (%APPDATA%\Liyamu)
SectionEnd
