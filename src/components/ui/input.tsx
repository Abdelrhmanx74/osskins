import * as React from "react";

import { cn } from "@/lib/utils";

interface InputProps extends React.ComponentProps<"input"> {
  icon?: React.ReactNode;
  iconPlacement?: "left" | "right";
}

function Input({
  className,
  type,
  icon,
  iconPlacement = "left",
  style,
  ...props
}: InputProps) {
  const inputElement = (
    <input
      type={type}
      data-slot="input"
      className={cn(
        "file:text-foreground placeholder:text-muted-foreground selection:bg-primary selection:text-primary-foreground bg-background flex h-9 w-full min-w-0 rounded-md border px-3 py-1 text-base shadow-xs transition-[color,box-shadow] outline-none file:inline-flex file:h-7 file:border-0 file:bg-transparent file:text-sm file:font-medium disabled:pointer-events-none disabled:cursor-not-allowed disabled:opacity-50 md:text-sm",
        "focus-visible:border-ring focus-visible:ring-ring/50 focus-visible:ring-[3px]",
        "aria-invalid:ring-destructive/20 dark:aria-invalid:ring-destructive/40 aria-invalid:border-destructive",
        icon ? (iconPlacement === "left" ? "pl-9" : "pr-9") : "",
        className
      )}
      style={style}
      {...props}
    />
  );

  if (!icon) return inputElement;

  return (
    <div className="relative flex items-center w-full">
      {iconPlacement === "left" && (
        <span className="absolute left-2 flex items-center pointer-events-none text-muted-foreground">
          {icon}
        </span>
      )}
      {inputElement}
      {iconPlacement === "right" && (
        <span className="absolute right-2 flex items-center pointer-events-none text-muted-foreground">
          {icon}
        </span>
      )}
    </div>
  );
}

export { Input };
