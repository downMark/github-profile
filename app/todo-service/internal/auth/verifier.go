package auth

import (
	"context"
	"crypto/rsa"
	"encoding/base64"
	"encoding/binary"
	"encoding/json"
	"errors"
	"math/big"
	"net/http"
	"strings"
	"sync"
	"time"

	"github.com/downMark/github-profile/app/todo-service/internal/domain"
	"github.com/golang-jwt/jwt/v5"
)

type Verifier struct {
	issuer, audience, jwksURL string
	client                    *http.Client
	mu                        sync.RWMutex
	keys                      map[string]*rsa.PublicKey
}

type jwks struct {
	Keys []jwk `json:"keys"`
}
type jwk struct{ Kid, Kty, Alg, N, E string }

func New(issuer, audience, jwksURL string) *Verifier {
	return &Verifier{issuer: issuer, audience: audience, jwksURL: jwksURL, client: &http.Client{Timeout: 2 * time.Second}, keys: map[string]*rsa.PublicKey{}}
}

func (v *Verifier) Authenticate(ctx context.Context, authorization string) (string, error) {
	tokenValue, ok := strings.CutPrefix(authorization, "Bearer ")
	if !ok || tokenValue == "" {
		return "", domain.ErrUnauthorized
	}
	parsed, err := jwt.Parse(tokenValue, func(token *jwt.Token) (any, error) {
		if token.Method.Alg() != jwt.SigningMethodRS256.Alg() {
			return nil, domain.ErrUnauthorized
		}
		kid, _ := token.Header["kid"].(string)
		if kid == "" {
			return nil, domain.ErrUnauthorized
		}
		if key := v.key(kid); key != nil {
			return key, nil
		}
		if err := v.refresh(ctx); err != nil {
			return nil, err
		}
		if key := v.key(kid); key != nil {
			return key, nil
		}
		return nil, domain.ErrUnauthorized
	}, jwt.WithValidMethods([]string{"RS256"}), jwt.WithIssuer(v.issuer), jwt.WithAudience(v.audience), jwt.WithExpirationRequired())
	if err != nil {
		if errors.Is(err, domain.ErrAuthUnavailable) {
			return "", domain.ErrAuthUnavailable
		}
		return "", domain.ErrUnauthorized
	}
	if !parsed.Valid {
		return "", domain.ErrUnauthorized
	}
	return "Bearer " + tokenValue, nil
}

func (v *Verifier) key(kid string) *rsa.PublicKey {
	v.mu.RLock()
	defer v.mu.RUnlock()
	return v.keys[kid]
}

func (v *Verifier) refresh(ctx context.Context) error {
	req, err := http.NewRequestWithContext(ctx, http.MethodGet, v.jwksURL, nil)
	if err != nil {
		return domain.ErrAuthUnavailable
	}
	res, err := v.client.Do(req)
	if err != nil {
		return domain.ErrAuthUnavailable
	}
	defer res.Body.Close()
	if res.StatusCode/100 != 2 {
		return domain.ErrAuthUnavailable
	}
	var set jwks
	if json.NewDecoder(res.Body).Decode(&set) != nil {
		return domain.ErrAuthUnavailable
	}
	keys := map[string]*rsa.PublicKey{}
	for _, item := range set.Keys {
		if item.Kty != "RSA" || item.Alg != "RS256" {
			continue
		}
		n, nerr := base64.RawURLEncoding.DecodeString(item.N)
		e, eerr := base64.RawURLEncoding.DecodeString(item.E)
		if nerr != nil || eerr != nil || len(e) == 0 || len(e) > 4 {
			continue
		}
		buf := make([]byte, 4)
		copy(buf[4-len(e):], e)
		keys[item.Kid] = &rsa.PublicKey{N: new(big.Int).SetBytes(n), E: int(binary.BigEndian.Uint32(buf))}
	}
	if len(keys) == 0 {
		return domain.ErrAuthUnavailable
	}
	v.mu.Lock()
	v.keys = keys
	v.mu.Unlock()
	return nil
}
