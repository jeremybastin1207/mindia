package iam

import "golang.org/x/crypto/bcrypt"

type Password = string

func HashPassword(password string) (Password, error) {
	bytes, err := bcrypt.GenerateFromPassword([]byte(password), 14)
	return string(bytes), err
}

func CheckPasswordHash(password Password, hash Password) bool {
	err := bcrypt.CompareHashAndPassword([]byte(hash), []byte(password))
	return err == nil
}
