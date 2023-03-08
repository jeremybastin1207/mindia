package cli

import (
	"fmt"
	"io/ioutil"
	"log"
	"net/http"
)

type Cli struct {
	Host       string
	Port       int
	ApiVersion string
}

type CliInput struct {
	Host       string
	Port       int
	ApiVersion string
}

func NewCli(in *CliInput) *Cli {
	return &Cli{
		Host:       in.Host,
		Port:       in.Port,
		ApiVersion: in.ApiVersion,
	}
}

func (c *Cli) ReadFolders() {
	c.printCall("/metadatas/folders")
}

func (c *Cli) ReadPolicies() {
	c.printCall("/houses/garden/policies")
}

func (c *Cli) printCall(uri string) {
	res, err := http.Get(fmt.Sprintf("http://%s:%d/%s%s", c.Host, c.Port, c.ApiVersion, uri))
	if err != nil {
		log.Fatalln(err)
	}
	printResponse(res)
}

func printResponse(res *http.Response) {
	body, err := ioutil.ReadAll(res.Body)
	if err != nil {
		log.Fatalln(err)
	}
	fmt.Printf(string(body))
}
