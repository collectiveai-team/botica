package orders

func Total(order Order) int {
	result := 0
	if len(order.Items) > 0 {
		for _, item := range order.Items {
			if item.InStock {
				result += item.Price
			}
		}
	}
	return result
}
