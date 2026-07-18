package httpapi

import (
	"context"
	"encoding/json"
	"errors"
	"fmt"
	"io"
	"log/slog"
	"net/http"
	"strconv"
	"strings"

	"github.com/downMark/github-profile/app/todo-service/internal/domain"
	"github.com/downMark/github-profile/app/todo-service/internal/requestauth"
	"github.com/google/uuid"
)

const maxRequestBody = 1 << 20

type TodoService interface {
	Create(context.Context, uuid.UUID, domain.CreateInput) (domain.Todo, error)
	List(context.Context, uuid.UUID, uint32, uint32) (domain.ListResult, error)
	Get(context.Context, uuid.UUID, uuid.UUID) (domain.Todo, error)
	Update(context.Context, uuid.UUID, uuid.UUID, domain.UpdateInput) (domain.Todo, error)
	Delete(context.Context, uuid.UUID, uuid.UUID) error
}

type Authenticator interface {
	Authenticate(context.Context, string) (string, error)
}

type Handler struct {
	service       TodoService
	logger        *slog.Logger
	allowedOrigin string
	basePath      string
	auth          Authenticator
}

type errorResponse struct {
	Code    string `json:"code"`
	Message string `json:"message"`
}

type createRequest struct {
	Title       string  `json:"title"`
	Description *string `json:"description"`
}

type updateRequest struct {
	Title       *string         `json:"title"`
	Description json.RawMessage `json:"description"`
	Completed   *bool           `json:"completed"`
}

func New(service TodoService, auth Authenticator, logger *slog.Logger, allowedOrigin, basePath string) http.Handler {
	handler := &Handler{service: service, auth: auth, logger: logger, allowedOrigin: allowedOrigin, basePath: basePath}
	mux := http.NewServeMux()
	mux.HandleFunc(basePath+"/health", handler.health)
	mux.HandleFunc(basePath+"/health/todo", handler.health)
	mux.HandleFunc(basePath+"/api/users/{user_id}/todos", handler.collection)
	mux.HandleFunc(basePath+"/api/users/{user_id}/todos/{todo_id}", handler.item)
	return handler.cors(handler.recoverPanic(mux))
}

func (h *Handler) health(w http.ResponseWriter, r *http.Request) {
	if r.Method != http.MethodGet {
		h.methodNotAllowed(w)
		return
	}
	writeJSON(w, http.StatusOK, map[string]string{"status": "ok"})
}

func (h *Handler) collection(w http.ResponseWriter, r *http.Request) {
	if r.Method == http.MethodOptions {
		w.WriteHeader(http.StatusNoContent)
		return
	}
	r, ok := h.authenticate(w, r)
	if !ok {
		return
	}
	userID, ok := h.pathUUID(w, r, "user_id", "user id")
	if !ok {
		return
	}
	switch r.Method {
	case http.MethodPost:
		h.create(w, r, userID)
	case http.MethodGet:
		h.list(w, r, userID)
	default:
		h.methodNotAllowed(w)
	}
}

func (h *Handler) item(w http.ResponseWriter, r *http.Request) {
	if r.Method == http.MethodOptions {
		w.WriteHeader(http.StatusNoContent)
		return
	}
	r, ok := h.authenticate(w, r)
	if !ok {
		return
	}
	userID, ok := h.pathUUID(w, r, "user_id", "user id")
	if !ok {
		return
	}
	todoID, ok := h.pathUUID(w, r, "todo_id", "todo id")
	if !ok {
		return
	}
	switch r.Method {
	case http.MethodGet:
		h.get(w, r, userID, todoID)
	case http.MethodPatch:
		h.update(w, r, userID, todoID)
	case http.MethodDelete:
		h.delete(w, r, userID, todoID)
	default:
		h.methodNotAllowed(w)
	}
}

func (h *Handler) authenticate(w http.ResponseWriter, r *http.Request) (*http.Request, bool) {
	bearer, err := h.auth.Authenticate(r.Context(), r.Header.Get("Authorization"))
	if err != nil {
		h.writeError(w, err)
		return r, false
	}
	return r.WithContext(requestauth.WithBearer(r.Context(), bearer)), true
}

func (h *Handler) create(w http.ResponseWriter, r *http.Request, userID uuid.UUID) {
	var request createRequest
	if !decodeJSON(w, r, &request) {
		return
	}
	item, err := h.service.Create(r.Context(), userID, domain.CreateInput{Title: request.Title, Description: request.Description})
	if err != nil {
		h.writeError(w, err)
		return
	}
	writeJSON(w, http.StatusCreated, item)
}

func (h *Handler) list(w http.ResponseWriter, r *http.Request, userID uuid.UUID) {
	page, err := queryUint32(r, "page", 1)
	if err != nil {
		h.writeError(w, err)
		return
	}
	limit, err := queryUint32(r, "limit", 20)
	if err != nil {
		h.writeError(w, err)
		return
	}
	result, err := h.service.List(r.Context(), userID, page, limit)
	if err != nil {
		h.writeError(w, err)
		return
	}
	writeJSON(w, http.StatusOK, result)
}

func (h *Handler) get(w http.ResponseWriter, r *http.Request, userID, todoID uuid.UUID) {
	item, err := h.service.Get(r.Context(), userID, todoID)
	if err != nil {
		h.writeError(w, err)
		return
	}
	writeJSON(w, http.StatusOK, item)
}

func (h *Handler) update(w http.ResponseWriter, r *http.Request, userID, todoID uuid.UUID) {
	var request updateRequest
	if !decodeJSON(w, r, &request) {
		return
	}
	input := domain.UpdateInput{Title: request.Title, Completed: request.Completed}
	if request.Description != nil {
		input.DescriptionSet = true
		if string(request.Description) != "null" {
			var description string
			if err := json.Unmarshal(request.Description, &description); err != nil {
				writeJSON(w, http.StatusBadRequest, errorResponse{Code: "invalid_argument", Message: "description must be a string or null"})
				return
			}
			input.Description = &description
		}
	}
	item, err := h.service.Update(r.Context(), userID, todoID, input)
	if err != nil {
		h.writeError(w, err)
		return
	}
	writeJSON(w, http.StatusOK, item)
}

func (h *Handler) delete(w http.ResponseWriter, r *http.Request, userID, todoID uuid.UUID) {
	if err := h.service.Delete(r.Context(), userID, todoID); err != nil {
		h.writeError(w, err)
		return
	}
	w.WriteHeader(http.StatusNoContent)
}

func (h *Handler) pathUUID(w http.ResponseWriter, r *http.Request, name, label string) (uuid.UUID, bool) {
	value, err := uuid.Parse(r.PathValue(name))
	if err != nil {
		writeJSON(w, http.StatusBadRequest, errorResponse{Code: "invalid_argument", Message: label + " must be a valid UUID"})
		return uuid.Nil, false
	}
	return value, true
}

func (h *Handler) writeError(w http.ResponseWriter, err error) {
	switch {
	case errors.Is(err, domain.ErrInvalidInput):
		writeJSON(w, http.StatusBadRequest, errorResponse{Code: "invalid_argument", Message: strings.TrimPrefix(err.Error(), domain.ErrInvalidInput.Error()+": ")})
	case errors.Is(err, domain.ErrUserNotFound):
		writeJSON(w, http.StatusNotFound, errorResponse{Code: "user_not_found", Message: "user not found"})
	case errors.Is(err, domain.ErrTodoNotFound):
		writeJSON(w, http.StatusNotFound, errorResponse{Code: "todo_not_found", Message: "todo not found"})
	case errors.Is(err, domain.ErrProfileUnavailable):
		writeJSON(w, http.StatusServiceUnavailable, errorResponse{Code: "profile_unavailable", Message: "profile service is temporarily unavailable"})
	case errors.Is(err, domain.ErrUnauthorized):
		writeJSON(w, http.StatusUnauthorized, errorResponse{Code: "unauthorized", Message: "unauthorized"})
	case errors.Is(err, domain.ErrAuthUnavailable):
		writeJSON(w, http.StatusServiceUnavailable, errorResponse{Code: "auth_unavailable", Message: "authentication service is temporarily unavailable"})
	default:
		h.logger.Error("todo request failed", "error", err)
		writeJSON(w, http.StatusInternalServerError, errorResponse{Code: "internal_error", Message: "internal server error"})
	}
}

func (h *Handler) methodNotAllowed(w http.ResponseWriter) {
	writeJSON(w, http.StatusMethodNotAllowed, errorResponse{Code: "method_not_allowed", Message: "method not allowed"})
}

func (h *Handler) cors(next http.Handler) http.Handler {
	return http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		w.Header().Set("Access-Control-Allow-Origin", h.allowedOrigin)
		w.Header().Set("Vary", "Origin")
		w.Header().Set("Access-Control-Allow-Headers", "Content-Type, Authorization")
		w.Header().Set("Access-Control-Allow-Credentials", "true")
		w.Header().Set("Access-Control-Allow-Methods", "GET, POST, PATCH, DELETE, OPTIONS")
		next.ServeHTTP(w, r)
	})
}

func (h *Handler) recoverPanic(next http.Handler) http.Handler {
	return http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		defer func() {
			if recovered := recover(); recovered != nil {
				h.logger.Error("panic while serving todo request", "panic", recovered)
				writeJSON(w, http.StatusInternalServerError, errorResponse{Code: "internal_error", Message: "internal server error"})
			}
		}()
		next.ServeHTTP(w, r)
	})
}

func decodeJSON(w http.ResponseWriter, r *http.Request, target any) bool {
	r.Body = http.MaxBytesReader(w, r.Body, maxRequestBody)
	decoder := json.NewDecoder(r.Body)
	decoder.DisallowUnknownFields()
	if err := decoder.Decode(target); err != nil {
		writeJSON(w, http.StatusBadRequest, errorResponse{Code: "invalid_argument", Message: "request body must be valid JSON"})
		return false
	}
	if err := decoder.Decode(&struct{}{}); !errors.Is(err, io.EOF) {
		writeJSON(w, http.StatusBadRequest, errorResponse{Code: "invalid_argument", Message: "request body must contain one JSON object"})
		return false
	}
	return true
}

func queryUint32(r *http.Request, name string, fallback uint32) (uint32, error) {
	value := r.URL.Query().Get(name)
	if value == "" {
		return fallback, nil
	}
	number, err := strconv.ParseUint(value, 10, 32)
	if err != nil {
		return 0, fmt.Errorf("%w: %s must be a positive integer", domain.ErrInvalidInput, name)
	}
	return uint32(number), nil
}

func writeJSON(w http.ResponseWriter, status int, body any) {
	w.Header().Set("Content-Type", "application/json")
	w.WriteHeader(status)
	if err := json.NewEncoder(w).Encode(body); err != nil {
		slog.Error("encode response", "error", err)
	}
}
