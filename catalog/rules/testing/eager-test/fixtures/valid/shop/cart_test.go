package shop

import (
	"testing"

	"github.com/stretchr/testify/assert"
)

func TestAdd(t *testing.T) {
	cart := NewCart()
	cart.Add(1)
	assert.Equal(t, 1, len(cart.Items()))
	if cart.Total() != 1 {
		t.Errorf("total %d", cart.Total())
	}
}
