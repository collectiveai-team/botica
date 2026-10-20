module.exports.keep = (secret) => {
  localStorage.setItem("apiSecret", secret);
};
