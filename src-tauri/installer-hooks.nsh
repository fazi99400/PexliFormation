; PexliFormation NSIS installer hooks.
;
; On uninstall, remove the app-private toolchain cache we created at runtime
; (the isolated CARGO_HOME under the user's local app data), so uninstalling
; leaves nothing behind. We never touch the user's own ~/.cargo or system Rust.

!macro NSIS_HOOK_POSTUNINSTALL
  ; %LOCALAPPDATA%\<identifier>\toolchain  (identifier = com.pexli.formation)
  RMDir /r "$LOCALAPPDATA\com.pexli.formation\toolchain"
  RMDir /r "$LOCALAPPDATA\com.pexli.formation"
!macroend
