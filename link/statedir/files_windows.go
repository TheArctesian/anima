package statedir

import (
	"errors"
	"io/fs"
	"os"
	"path/filepath"
	"unsafe"

	"golang.org/x/sys/windows"
)

func currentSID() (*windows.SID, error) {
	user, err := windows.GetCurrentProcessToken().GetTokenUser()
	if err != nil {
		return nil, err
	}
	return user.User.Sid, nil
}

// System administrators can take ownership irrespective of a file's ACL.
func trustedSID(sid, user *windows.SID) bool {
	return sid.Equals(user) || sid.IsWellKnown(windows.WinLocalSystemSid) || sid.IsWellKnown(windows.WinBuiltinAdministratorsSid)
}

// Windows installs the volume root with TrustedInstaller as its owner. That
// system service is trusted for ancestors, but not for private link state.
func trustedOwner(sid, user *windows.SID, private bool) bool {
	if trustedSID(sid, user) {
		return true
	}
	if private {
		return false
	}
	installer, _, _, err := windows.LookupSID("", `NT SERVICE\TrustedInstaller`)
	return err == nil && sid.Equals(installer)
}

// Ancestors may be readable, but another user must not be able to replace a child.
func accessAllows(path string, private bool) bool {
	user, err := currentSID()
	if err != nil {
		return false
	}
	sd, err := windows.GetNamedSecurityInfo(path, windows.SE_FILE_OBJECT, windows.OWNER_SECURITY_INFORMATION|windows.DACL_SECURITY_INFORMATION)
	if err != nil {
		return false
	}
	owner, _, err := sd.Owner()
	if err != nil || !trustedOwner(owner, user, private) {
		return false
	}
	acl, _, err := sd.DACL()
	if err != nil || acl == nil {
		return false
	}
	for i := uint32(0); i < uint32(acl.AceCount); i++ {
		var ace *windows.ACCESS_ALLOWED_ACE
		if windows.GetAce(acl, i, &ace) != nil || !aceAllows(ace, user, private) {
			return false
		}
	}
	return true
}

// Inherit-only rules apply to children; deny rules cannot grant access.
func aceAllows(ace *windows.ACCESS_ALLOWED_ACE, user *windows.SID, private bool) bool {
	if ace.Header.AceFlags&windows.INHERIT_ONLY_ACE != 0 || ace.Header.AceType == windows.ACCESS_DENIED_ACE_TYPE {
		return true
	}
	if ace.Header.AceType != windows.ACCESS_ALLOWED_ACE_TYPE {
		return false
	}
	sid := (*windows.SID)(unsafe.Pointer(&ace.SidStart))
	replace := windows.ACCESS_MASK(windows.DELETE | windows.WRITE_DAC | windows.WRITE_OWNER | 0x0040 | windows.GENERIC_ALL | windows.GENERIC_WRITE)
	return trustedSID(sid, user) || (!private && ace.Mask&replace == 0)
}

func makeDirectory(path string) error {
	if _, err := os.Lstat(path); err == nil {
		return nil
	} else if !errors.Is(err, fs.ErrNotExist) {
		return err
	}
	if err := foldersAbove(path); err != nil {
		return err
	}
	if err := os.MkdirAll(path, 0o700); err != nil {
		return err
	}
	user, err := currentSID()
	if err != nil {
		return err
	}
	sd, err := windows.SecurityDescriptorFromString("D:P(A;OICI;FA;;;" + user.String() + ")(A;OICI;FA;;;SY)(A;OICI;FA;;;BA)")
	if err != nil {
		return err
	}
	acl, _, err := sd.DACL()
	if err != nil {
		return err
	}
	return windows.SetNamedSecurityInfo(path, windows.SE_FILE_OBJECT, windows.DACL_SECURITY_INFORMATION|windows.PROTECTED_DACL_SECURITY_INFORMATION, nil, nil, acl, nil)
}

func foldersAbove(path string) error {
	for above := filepath.Dir(path); ; above = filepath.Dir(above) {
		stat, err := os.Lstat(above)
		if err != nil {
			return err
		}
		if !stat.IsDir() || stat.Mode()&os.ModeSymlink != 0 || !accessAllows(above, false) {
			return ErrExposed
		}
		if above == filepath.Dir(above) {
			return nil
		}
	}
}

func ownerOnly(stat fs.FileInfo, path string) bool {
	return stat.Mode()&os.ModeSymlink == 0 && accessAllows(path, true)
}

func lockFile(file *os.File) error {
	return windows.LockFileEx(windows.Handle(file.Fd()), windows.LOCKFILE_EXCLUSIVE_LOCK|windows.LOCKFILE_FAIL_IMMEDIATELY, 0, 1, 0, &windows.Overlapped{})
}

func readFile(path string) (*os.File, error) {
	name, err := windows.UTF16PtrFromString(path)
	if err != nil {
		return nil, err
	}
	handle, err := windows.CreateFile(name, windows.GENERIC_READ, windows.FILE_SHARE_READ|windows.FILE_SHARE_WRITE|windows.FILE_SHARE_DELETE, nil, windows.OPEN_EXISTING, windows.FILE_FLAG_OPEN_REPARSE_POINT, 0)
	if err != nil {
		return nil, err
	}
	var details windows.ByHandleFileInformation
	if err := windows.GetFileInformationByHandle(handle, &details); err != nil || details.FileAttributes&windows.FILE_ATTRIBUTE_REPARSE_POINT != 0 {
		windows.CloseHandle(handle)
		return nil, ErrExposed
	}
	return os.NewFile(uintptr(handle), path), nil
}

// Windows cannot flush a directory handle. Each file is flushed before its rename.
func syncDirectory(*os.File) error { return nil }

// CheckFile refuses anything other than a private regular file of this directory.
// It checks the opened file and never reads or prints its contents.
func (d *Dir) CheckFile(name string) error {
	if name == "." || name == ".." || filepath.Base(name) != name {
		return ErrExposed
	}
	path := filepath.Join(d.path, name)
	file, err := readFile(path)
	if err != nil {
		return err
	}
	defer file.Close()
	stat, err := file.Stat()
	if err != nil {
		return err
	}
	if !stat.Mode().IsRegular() || !ownerOnly(stat, path) {
		return ErrExposed
	}
	return nil
}
