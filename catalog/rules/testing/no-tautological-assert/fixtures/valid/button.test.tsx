test("label", () => {
  const label = <span>Pay</span>;
  expect(label.props.children).toEqual("Pay");
});
