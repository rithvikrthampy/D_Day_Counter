!macro customInstall
  ExecWait '"$INSTDIR\D-daycounter.exe" --register-mcp'
!macroend

!macro customUnInstall
  ExecWait '"$INSTDIR\D-daycounter.exe" --unregister-mcp'
!macroend
