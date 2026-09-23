!macro NSIS_HOOK_PREUNINSTALL
  ; Close rshot first: a running one keeps its keyboard hook and its own copy of the settings.
  !insertmacro CheckIfAppIsRunning "${MAINBINARYNAME}.exe" "${PRODUCTNAME}"
  ; Give PrtScn / Win+Shift+S back to Windows before removing rshot. Upgrades run this too, so the
  ; takeover stays on (--keep-consent) and the next launch takes the keys again.
  ExecWait '"$INSTDIR\${MAINBINARYNAME}.exe" restore-shortcuts --keep-consent'
!macroend
