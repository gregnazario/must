package main

import (
	"fmt"
	"net/http"
	"os"
)

var version = "dev"

func handler(w http.ResponseWriter, r *http.Request) {
	fmt.Fprintf(w, "my-go-service %s\n", version)
}

func main() {
	port := os.Getenv("PORT")
	if port == "" {
		port = "8080"
	}

	http.HandleFunc("/", handler)
	fmt.Printf("listening on :%s\n", port)
	if err := http.ListenAndServe(":"+port, nil); err != nil {
		fmt.Fprintf(os.Stderr, "error: %v\n", err)
		os.Exit(1)
	}
}
