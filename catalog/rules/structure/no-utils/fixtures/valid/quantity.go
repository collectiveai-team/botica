package orders

func clamp(n, low, high int) int {
	return min(max(n, low), high)
}
