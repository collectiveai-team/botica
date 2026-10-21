const jsonwebtoken = require("jsonwebtoken");

module.exports.claimsOf = (token) => jsonwebtoken.decode(token);
