package apiserver

import (
	"bytes"
	"encoding/json"
	"fmt"
	"io"
	"mime/multipart"
	"mindia/folder"
	"net/http"
)

const apiVersion = "v0"

type ApiServerConfig struct {
	Port int `yaml:"api_server"`
}

type ApiServer struct {
	folders          folder.FoldersMap
	routes           []route
	*ApiServerConfig `yaml:",inline"`
}

func NewApiServer(config *ApiServerConfig) *ApiServer {
	return &ApiServer{
		folders:         folder.FoldersMap{},
		ApiServerConfig: config,
	}
}

func (s *ApiServer) AddFolder(f *folder.Folder) {
	s.folders[f.Dir] = f
}

func (s *ApiServer) Serve() {
	s.routes = []route{
		newRoute("GET", "/metadatas/folders", s.handleReadFolders),
		newRoute("GET", "(/.*)/list", s.handleReadFolder),
		newRoute("GET", "(/.*)/download/(.*)", s.handleDownload),
		newRoute("POST", "(/.*)/upload", s.handleUpload),
		newRoute("DELETE", "(/.*)/file/(.*)", s.handleDelete),
	}

	fmt.Printf("listening on port: %d\n", s.Port)
	http.ListenAndServe(fmt.Sprintf("127.0.0.1:%d", s.Port), http.HandlerFunc(s.router))
}

func (s *ApiServer) handleReadFolders(w http.ResponseWriter, r *http.Request) {
	var folders []folder.Folder
	for _, folder := range s.folders {
		folders = append(folders, *folder)
	}
	writeJSON(w, folders)
}

func (s *ApiServer) handleReadFolder(w http.ResponseWriter, r *http.Request) {
	files, _ := s.folders[getFolder(r)].ReadAll()
	writeJSON(w, files)
}

func (s *ApiServer) handleDownload(w http.ResponseWriter, r *http.Request) {
	bytes, err := s.folders[getFolder(r)].Download(getField(r, 1))
	if err != nil {
		http.Error(w, err.Error(), http.StatusUnprocessableEntity)
		return
	}
	writeBytes(w, bytes)
}

func (s *ApiServer) handleUpload(w http.ResponseWriter, r *http.Request) {
	folder := getFolder(r)

	err := r.ParseMultipartForm(10 << 20) // Maximum upload of 10 MB files
	if err != nil {
		http.Error(w, err.Error(), http.StatusUnprocessableEntity)
		return
	}

	file, handler, err := r.FormFile("file")
	if err != nil {
		http.Error(w, err.Error(), http.StatusUnprocessableEntity)
		return
	}

	defer func(file multipart.File) {
		err := file.Close()
		if err != nil {
			http.Error(w, err.Error(), http.StatusInternalServerError)
			return
		}
	}(file)

	buf := bytes.NewBuffer(nil)
	if _, err := io.Copy(buf, file); err != nil {
		http.Error(w, err.Error(), http.StatusInternalServerError)
		return
	}

	err = s.folders[folder].Upload(handler.Filename, buf.Bytes())
	if err != nil {
		http.Error(w, err.Error(), http.StatusInternalServerError)
		return
	}

	http.Error(w, "File uploaded", http.StatusOK)
}

func (s *ApiServer) handleDelete(w http.ResponseWriter, r *http.Request) {
	err := s.folders[getFolder(r)].DeleteOne(getField(r, 1))
	if err != nil {
		http.Error(w, err.Error(), http.StatusUnprocessableEntity)
		return
	}
	_, err = fmt.Fprintf(w, "Successfully Deleted File\n")
	if err != nil {
		http.Error(w, err.Error(), http.StatusInternalServerError)
		return
	}
	w.WriteHeader(http.StatusOK)
}

func writeJSON(w http.ResponseWriter, obj interface{}) {
	jsonContent, err := json.MarshalIndent(obj, "", "	")
	if err != nil {
		http.Error(w, fmt.Sprintf("error building the response, %v", err), http.StatusInternalServerError)
		return
	}
	w.WriteHeader(http.StatusOK)
	w.Header().Set("Content-Type", "application/json")
	w.Write(jsonContent)
}

func writeBytes(w http.ResponseWriter, bytes []byte) {
	w.WriteHeader(http.StatusOK)
	w.Header().Set("Content-Type", "application/octet-stream")

	_, err := w.Write(bytes)
	if err != nil {
		http.Error(w, err.Error(), http.StatusUnprocessableEntity)
		return
	}
}
