import { render, screen, cleanup } from "@testing-library/react";
import { afterEach, expect, it } from "vitest";
import { Code } from "./common";

afterEach(cleanup);
it("renders untrusted source as escaped text with bounded lexical markup", () => {
  const source = '<img src=x onerror="alert(1)">';
  const { container } = render(<Code text={source} start={42} />);
  expect(container.querySelector("img")).toBeNull();
  expect(container.textContent).toContain(source);
  expect(screen.getByText("42")).toBeInTheDocument();
});
