package commands

import (
	"encoding/json"
	"fmt"
	"net/http"

	"github.com/chapmanjacobd/syncweb/internal/models"
)

// writeJSON writes a JSON response with the given status code.
// It ignores encoding errors as the connection may already be broken.
func writeJSON(w http.ResponseWriter, status int, data any) {
	w.Header().Set("Content-Type", "application/json")
	w.WriteHeader(status)
	if err := json.NewEncoder(w).Encode(data); err != nil {
		// Connection may be broken, nothing we can do
		return
	}
}

// writeOK writes a JSON response with status 200.
func writeOK(w http.ResponseWriter, data any) {
	writeJSON(w, http.StatusOK, data)
}

// writeAccepted writes a text response with status 202.
func writeAccepted(w http.ResponseWriter, message string) {
	w.WriteHeader(http.StatusAccepted)
	if _, err := fmt.Fprintln(w, message); err != nil {
		// Connection may be broken, nothing we can do
		return
	}
}

// writeError writes an error JSON response with the given status code.
func writeError(w http.ResponseWriter, status int, message string) {
	writeJSON(w, status, models.ErrorResponse{Error: message})
}

// writeServiceUnavailable writes a service unavailable error.
func writeServiceUnavailable(w http.ResponseWriter) {
	writeError(w, http.StatusServiceUnavailable, "Syncweb not configured or offline")
}

// writeBadRequest writes a bad request error.
func writeBadRequest(w http.ResponseWriter, message string) {
	writeError(w, http.StatusBadRequest, message)
}

// writeInternalServerError writes an internal server error.
func writeInternalServerError(w http.ResponseWriter, message string) {
	writeError(w, http.StatusInternalServerError, message)
}

// writeMethodNotAllowed writes a method not allowed error.
func writeMethodNotAllowed(w http.ResponseWriter) {
	writeError(w, http.StatusMethodNotAllowed, "Method not allowed")
}

// decodeJSON decodes a JSON request body into the given target.
func decodeJSON(r *http.Request, target any) error {
	if err := json.NewDecoder(r.Body).Decode(target); err != nil {
		return fmt.Errorf("invalid request body: %w", err)
	}
	return nil
}
