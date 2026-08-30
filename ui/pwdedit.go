package ui

import (
	"syscall"
	"unsafe"

	"github.com/lxn/win"
)

var (
	user32               = syscall.NewLazyDLL("user32.dll")
	kernel32             = syscall.NewLazyDLL("kernel32.dll")
	procSetWindowLongPtr = user32.NewProc("SetWindowLongPtrW")
	procGetWindowLongPtr = user32.NewProc("GetWindowLongPtrW")
	procCallWindowProc   = user32.NewProc("CallWindowProcW")
)

const (
	WM_KEYDOWN = 0x0100
	WM_PASTE   = 0x0302
)

type editUserData struct {
	origProc uintptr
}

func pasteToEdit(hwnd win.HWND) {
	win.SendMessage(hwnd, WM_PASTE, 0, 0)
}

func subclassEditSubclass(hwnd win.HWND, msg uint32, wParam, lParam uintptr) uintptr {
	ud := (*editUserData)(unsafe.Pointer(win.GetWindowLongPtr(hwnd, -21)))

	if msg == WM_KEYDOWN {
		if wParam == 0x16 && (int32(win.GetKeyState(0x11))&0x8000 != 0) {
			pasteToEdit(hwnd)
			return 0
		}
	}

	if ud != nil && ud.origProc != 0 {
		ret, _, _ := procCallWindowProc.Call(ud.origProc, uintptr(hwnd), uintptr(msg), wParam, lParam)
		return ret
	}
	return win.DefWindowProc(hwnd, msg, wParam, lParam)
}

func subclassEdit(hwnd win.HWND) {
	ud := &editUserData{}
	udPtr := uintptr(unsafe.Pointer(ud))

	win.SetWindowLongPtr(hwnd, -21, udPtr)

	ret := win.GetWindowLongPtr(hwnd, -4)
	ud.origProc = ret

	win.SetWindowLongPtr(hwnd, -4, syscall.NewCallback(subclassEditSubclass))
}

func createPasswordEdit(parent win.HWND, x, y, width, height int32) win.HWND {
	hInstance := win.GetModuleHandle(nil)

	className, _ := syscall.UTF16PtrFromString("EDIT")
	windowName, _ := syscall.UTF16PtrFromString("")

	style := uint32(win.WS_CHILD | win.WS_VISIBLE | win.WS_BORDER | win.ES_AUTOHSCROLL | win.ES_PASSWORD)

	hwnd := win.CreateWindowEx(
		0,
		className,
		windowName,
		style,
		x, y, width, height,
		parent,
		0,
		hInstance,
		nil,
	)

	if hwnd != 0 {
		hFont := win.GetStockObject(win.DEFAULT_GUI_FONT)
		win.SendMessage(hwnd, win.WM_SETFONT, uintptr(hFont), 1)
		subclassEdit(hwnd)
	}

	return hwnd
}

func getEditText(hwnd win.HWND) string {
	length := int(win.SendMessage(hwnd, 0x000E, 0, 0))
	if length == 0 {
		return ""
	}
	buf := make([]uint16, length+1)
	win.SendMessage(hwnd, 0x000D, uintptr(length+1), uintptr(unsafe.Pointer(&buf[0])))
	return syscall.UTF16ToString(buf)
}

func setEditText(hwnd win.HWND, text string) {
	textPtr, _ := syscall.UTF16PtrFromString(text)
	win.SendMessage(hwnd, 0x000C, 0, uintptr(unsafe.Pointer(textPtr)))
}
