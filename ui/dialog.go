package ui

import (
	"syscall"
	"unsafe"

	"github.com/lxn/walk"
	"github.com/lxn/walk/declarative"

	"statusbar/credential"
)

var (
	procGetClipboardData = user32.NewProc("GetClipboardData")
	procSetClipboardData = user32.NewProc("SetClipboardData")
	procOpenClipboard    = user32.NewProc("OpenClipboard")
	procCloseClipboard   = user32.NewProc("CloseClipboard")
	procEmptyClipboard   = user32.NewProc("EmptyClipboard")
	procGlobalLock       = kernel32.NewProc("GlobalLock")
	procGlobalUnlock     = kernel32.NewProc("GlobalUnlock")
	procGlobalAlloc      = kernel32.NewProc("GlobalAlloc")
)

const (
	CF_UNICODETEXT = 13
	GMEM_MOVEABLE  = 0x0002
)

func pasteFromClipboard() string {
	procOpenClipboard.Call(0)
	defer procCloseClipboard.Call(0)

	ret, _, _ := procGetClipboardData.Call(CF_UNICODETEXT)
	if ret == 0 {
		return ""
	}

	lockRet, _, _ := procGlobalLock.Call(ret)
	if lockRet == 0 {
		return ""
	}
	defer procGlobalUnlock.Call(ret)

	p := lockRet
	var chars []uint16
	for {
		ch := *(*uint16)(unsafe.Pointer(p))
		if ch == 0 {
			break
		}
		chars = append(chars, ch)
		p += 2
	}

	return syscall.UTF16ToString(chars)
}

func showSettingsDialog(owner walk.Form) bool {
	var akiID, akiSecret *walk.LineEdit
	var eURL, eUser, ePass *walk.LineEdit
	var dsKey *walk.LineEdit
	var dlgRef *walk.Dialog

	var initAKIID, initAKISecret, initESXiURL, initESXiUser, initESXiPass, initDSKey string
	if c := credential.Global().GetAliyun(); c != nil {
		initAKIID = c.AccessKeyID
		initAKISecret = c.AccessKeySecret
	}
	if c := credential.Global().GetESXi(); c != nil {
		initESXiURL = c.URL
		initESXiUser = c.User
		initESXiPass = c.Password
	}
	if c := credential.Global().GetDeepSeek(); c != nil {
		initDSKey = c.APIKey
	}

	pasteText := func(le *walk.LineEdit) {
		text := pasteFromClipboard()
		if text != "" {
			le.SetText(text)
		}
	}

	dlg := declarative.Dialog{
		AssignTo:  &dlgRef,
		Title:     "账户与密钥设置",
		Size:      declarative.Size{Width: 420, Height: 340},
		FixedSize: true,
		Layout:    declarative.VBox{Margins: declarative.Margins{Left: 12, Top: 8, Right: 12, Bottom: 8}},
		Children: []declarative.Widget{
			declarative.TabWidget{
				Pages: []declarative.TabPage{
					{
						Title: "阿里云",
						Content: declarative.Composite{
							Layout: declarative.VBox{Margins: declarative.Margins{Left: 8, Top: 8, Right: 8, Bottom: 8}},
							Children: []declarative.Widget{
								declarative.Label{Text: "AccessKey ID:"},
								declarative.LineEdit{AssignTo: &akiID, Text: initAKIID, MinSize: declarative.Size{Width: 360, Height: 24}},
								declarative.Label{Text: "AccessKey Secret:"},
								declarative.Composite{
									Layout: declarative.HBox{},
									Children: []declarative.Widget{
										declarative.LineEdit{AssignTo: &akiSecret, Text: initAKISecret, MinSize: declarative.Size{Width: 280, Height: 24}, PasswordMode: true},
										declarative.PushButton{
											Text: "粘贴",
											OnClicked: func() { pasteText(akiSecret) },
										},
									},
								},
							},
						},
					},
					{
						Title: "ESXi",
						Content: declarative.Composite{
							Layout: declarative.VBox{Margins: declarative.Margins{Left: 8, Top: 8, Right: 8, Bottom: 8}},
							Children: []declarative.Widget{
								declarative.Label{Text: "访问地址 (如 https://192.168.1.100:443):"},
								declarative.LineEdit{AssignTo: &eURL, Text: initESXiURL, MinSize: declarative.Size{Width: 360, Height: 24}},
								declarative.Label{Text: "用户名:"},
								declarative.LineEdit{AssignTo: &eUser, Text: initESXiUser, MinSize: declarative.Size{Width: 360, Height: 24}},
								declarative.Label{Text: "密码:"},
								declarative.Composite{
									Layout: declarative.HBox{},
									Children: []declarative.Widget{
										declarative.LineEdit{AssignTo: &ePass, Text: initESXiPass, MinSize: declarative.Size{Width: 280, Height: 24}, PasswordMode: true},
										declarative.PushButton{
											Text: "粘贴",
											OnClicked: func() { pasteText(ePass) },
										},
									},
								},
							},
						},
					},
					{
						Title: "DeepSeek",
						Content: declarative.Composite{
							Layout: declarative.VBox{Margins: declarative.Margins{Left: 8, Top: 8, Right: 8, Bottom: 8}},
							Children: []declarative.Widget{
								declarative.Label{Text: "API Key:"},
								declarative.Composite{
									Layout: declarative.HBox{},
									Children: []declarative.Widget{
										declarative.LineEdit{AssignTo: &dsKey, Text: initDSKey, MinSize: declarative.Size{Width: 280, Height: 24}, PasswordMode: true},
										declarative.PushButton{
											Text: "粘贴",
											OnClicked: func() { pasteText(dsKey) },
										},
									},
								},
							},
						},
					},
				},
			},
			declarative.Composite{
				Layout: declarative.HBox{},
				Children: []declarative.Widget{
					declarative.HSpacer{},
					declarative.PushButton{
						Text: "保存",
						OnClicked: func() {
							id := akiID.Text()
							secret := akiSecret.Text()
							if id != "" && secret != "" {
								credential.Global().SetAliyun(&credential.AliyunCred{
									AccessKeyID:     id,
									AccessKeySecret: secret,
								})
							} else {
								credential.Global().SetAliyun(nil)
							}

							u := eURL.Text()
							un := eUser.Text()
							p := ePass.Text()
							if u != "" && un != "" && p != "" {
								credential.Global().SetESXi(&credential.ESXiCred{
									URL:      u,
									User:     un,
									Password: p,
								})
							} else {
								credential.Global().SetESXi(nil)
							}

							k := dsKey.Text()
							if k != "" {
								credential.Global().SetDeepSeek(&credential.DeepSeekCred{
									APIKey: k,
								})
							} else {
								credential.Global().SetDeepSeek(nil)
							}

							if dlgRef != nil {
								dlgRef.Accept()
							}
						},
					},
					declarative.PushButton{
						Text: "取消",
						OnClicked: func() {
							if dlgRef != nil {
								dlgRef.Cancel()
							}
						},
					},
				},
			},
		},
	}

	result, err := dlg.Run(owner)
	return err == nil && result == 1
}
