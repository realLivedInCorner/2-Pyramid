; 2-Pyramid NSIS Include Script
; 添加右键菜单注册表项

!macro NSIS_HOOK_INSTALL
  ; 添加右键菜单到 HKCU（当前用户）
  WriteRegStr HKCU "Software\Classes\.zip\shell\2-Pyramid" "" "转换版本至(&C)"
  WriteRegStr HKCU "Software\Classes\.zip\shell\2-Pyramid" "Icon" "$\"$INSTDIR\${MAINBINARYNAME}.exe$\",0"
  WriteRegStr HKCU "Software\Classes\.zip\shell\2-Pyramid\command" "" "$\"$INSTDIR\${MAINBINARYNAME}.exe$\" --convert $\"%1$\""
  
  ; 同时添加到 HKCR 的 SystemFileAssociations（兼容 Win10/Win11）
  WriteRegStr HKCR "SystemFileAssociations\.zip\shell\2-Pyramid" "" "转换版本至(&C)"
  WriteRegStr HKCR "SystemFileAssociations\.zip\shell\2-Pyramid" "Icon" "$\"$INSTDIR\${MAINBINARYNAME}.exe$\",0"
  WriteRegStr HKCR "SystemFileAssociations\.zip\shell\2-Pyramid\command" "" "$\"$INSTDIR\${MAINBINARYNAME}.exe$\" --convert $\"%1$\""
!macroend

!macro NSIS_HOOK_UNINSTALL
  ; 删除 HKCU 中的右键菜单
  DeleteRegKey HKCU "Software\Classes\.zip\shell\2-Pyramid"
  
  ; 删除 HKCR 中的右键菜单
  DeleteRegKey HKCR "SystemFileAssociations\.zip\shell\2-Pyramid"
!macroend
