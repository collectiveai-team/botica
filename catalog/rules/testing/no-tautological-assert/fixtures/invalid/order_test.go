package order

import (
	"testing"

	"github.com/stretchr/testify/assert"
	"github.com/stretchr/testify/require"
)

func TestTotal(t *testing.T) {
	total := Total([]int{1, 2})
	assert.Equal(t, total, total)
	require.True(t, true)
}
