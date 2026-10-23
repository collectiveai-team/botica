terraform {
  required_providers {
    random = {
      source  = "hashicorp/random"
      version = "~> 3.7"
    }
  }

  backend "local" {}
}

module "queue" {
  source = "../modules/queue"
}
