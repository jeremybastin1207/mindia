package cli

import "testing"

func TestReadPolicies(t *testing.T) {
	cli := NewCli(&CliInput{
		Host:       "127.0.0.1",
		Port:       3500,
		ApiVersion: "v0",
	})
	cli.ReadPolicies()
}
