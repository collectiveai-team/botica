// The button of the cart.
'use client'

import { loadStripe } from "@stripe/stripe-js";

export const stripe = loadStripe(process.env.STRIPE_SECRET_KEY ?? "");
