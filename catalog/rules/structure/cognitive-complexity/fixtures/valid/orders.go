package orders

func Total(order Order) int {
	result := 0
	for _, item := range order.Items {
		result += priceOf(item)
	}
	return result
}

func priceOf(item Item) int {
	if item.InStock {
		return item.Price
	}
	return 0
}
