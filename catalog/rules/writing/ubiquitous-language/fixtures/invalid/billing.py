def invoice_for(client: Customer) -> Invoice:
    return Invoice(customer=client, lines=client.orders)


def bill_buyer(account: Customer) -> Invoice:
    return invoice_for(account)
