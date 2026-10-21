def invoice_for(customer: Customer) -> Invoice:
    return Invoice(customer=customer, lines=customer.orders)
