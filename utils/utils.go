package utils

import (
	"fmt"
	"os"
	"path/filepath"
	"regexp"
	"strings"
)

func ExitErrorf(msg string, args ...interface{}) {
	fmt.Fprintf(os.Stderr, msg+"\n", args...)
	os.Exit(1)
}

func LastSlash(path string) int {
	return strings.LastIndex(path, "/")
}

func JoinPath(elems ...string) string {
	return filepath.ToSlash(filepath.Join(elems...))
}

func IsValidUUID(uuid string) bool {
	r := regexp.MustCompile("^[a-fA-F0-9]{8}-[a-fA-F0-9]{4}-4[a-fA-F0-9]{3}-[8|9|aA|bB][a-fA-F0-9]{3}-[a-fA-F0-9]{12}$")
	return r.MatchString(uuid)
}

func NameWithoutExt(Name string) string {
	return Name[:len(Name)-len(filepath.Ext(Name))]
}

func ToArray[T any](mp map[string]*T) []*T {
	var arr []*T
	for _, p := range mp {
		arr = append(arr, p)
	}
	return arr
}
