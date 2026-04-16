import type { ButtonHTMLAttributes, ReactNode } from "react";
import { classNames } from "../lib/classNames";

type ButtonVariant = "primary" | "secondary" | "ghost";

type ButtonProps = ButtonHTMLAttributes<HTMLButtonElement> & {
  variant?: ButtonVariant;
  icon?: ReactNode;
};

export function Button({ className, variant = "secondary", icon, children, ...props }: ButtonProps) {
  return (
    <button className={classNames("button", `button-${variant}`, className)} {...props}>
      {icon}
      <span>{children}</span>
    </button>
  );
}
