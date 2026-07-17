package requestauth

import "context"

type bearerKey struct{}

func WithBearer(ctx context.Context, bearer string) context.Context {
	return context.WithValue(ctx, bearerKey{}, bearer)
}

func Bearer(ctx context.Context) (string, bool) {
	value, ok := ctx.Value(bearerKey{}).(string)
	return value, ok && value != ""
}
