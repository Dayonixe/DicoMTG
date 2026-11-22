!macro NSIS_HOOK_PREINSTALL
  ; The standard NSIS check offers to terminate other running instances.
  ${If} $UpdateMode == 1
  ${AndIf} ${FileExists} "$INSTDIR\${MAINBINARYNAME}.exe"
    !insertmacro RestartManager_StartSession $R0
    ${If} $R0 == ""
      Abort "Impossible de vérifier si DicoMTG est encore ouvert."
    ${EndIf}

    !insertmacro RestartManager_RegisterFile $R0 "$INSTDIR\${MAINBINARYNAME}.exe"
    ${If} $0 == 0
      System::Call 'RSTRTMGR::RmGetList(p R0, *i .r1, *i .r2, p 0, *i .r3) i .r0'
    ${EndIf}
    StrCpy $R1 $0
    !insertmacro RestartManager_EndSession $R0
    ${If} $R1 != 0
      Abort "Fermez les autres fenêtres de DicoMTG après la fin des opérations et relancez la mise à jour."
    ${EndIf}
  ${EndIf}
!macroend
