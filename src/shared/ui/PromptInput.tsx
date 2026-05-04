import * as React from "react";

import { cn } from "../lib/utils";
import { Textarea } from "./Textarea";

type PromptInputContextValue = {
  disabled: boolean;
  maxHeight: number | string;
  onSubmit?: () => void;
  setValue: (value: string) => void;
  textareaRef: React.RefObject<HTMLTextAreaElement | null>;
  value: string;
};

const PromptInputContext = React.createContext<PromptInputContextValue | null>(null);

function usePromptInputContext() {
  const context = React.useContext(PromptInputContext);
  if (!context) {
    throw new Error("PromptInput components must be rendered inside PromptInput");
  }
  return context;
}

type PromptInputProps = React.HTMLAttributes<HTMLDivElement> & {
  disabled?: boolean;
  maxHeight?: number | string;
  onSubmit?: () => void;
  onValueChange?: (value: string) => void;
  value: string;
};

const PromptInput = React.forwardRef<HTMLDivElement, PromptInputProps>(
  (
    {
      children,
      className,
      disabled = false,
      maxHeight = 240,
      onClick,
      onSubmit,
      onValueChange,
      value,
      ...props
    },
    ref,
  ) => {
    const textareaRef = React.useRef<HTMLTextAreaElement>(null);

    function handleClick(event: React.MouseEvent<HTMLDivElement>) {
      if (!disabled) {
        textareaRef.current?.focus();
      }
      onClick?.(event);
    }

    return (
      <PromptInputContext.Provider
        value={{
          disabled,
          maxHeight,
          onSubmit,
          setValue: onValueChange ?? (() => undefined),
          textareaRef,
          value,
        }}
      >
        <div
          ref={ref}
          className={cn(
            "grid cursor-text gap-3 rounded-lg border border-input bg-background p-3 shadow-sm transition-colors focus-within:ring-1 focus-within:ring-ring",
            disabled && "cursor-not-allowed opacity-60",
            className,
          )}
          onClick={handleClick}
          {...props}
        >
          {children}
        </div>
      </PromptInputContext.Provider>
    );
  },
);
PromptInput.displayName = "PromptInput";

type PromptInputTextareaProps = React.ComponentProps<typeof Textarea> & {
  disableAutosize?: boolean;
};

const PromptInputTextarea = React.forwardRef<HTMLTextAreaElement, PromptInputTextareaProps>(
  ({ className, disableAutosize = false, onKeyDown, ...props }, forwardedRef) => {
    const { disabled, maxHeight, onSubmit, setValue, textareaRef, value } = usePromptInputContext();

    const resize = React.useCallback(
      (element: HTMLTextAreaElement | null) => {
        if (!element || disableAutosize) return;
        element.style.height = "auto";
        if (typeof maxHeight === "number") {
          element.style.height = `${Math.min(element.scrollHeight, maxHeight)}px`;
        } else {
          element.style.height = `min(${element.scrollHeight}px, ${maxHeight})`;
        }
      },
      [disableAutosize, maxHeight],
    );

    const setRefs = React.useCallback(
      (element: HTMLTextAreaElement | null) => {
        textareaRef.current = element;
        if (typeof forwardedRef === "function") {
          forwardedRef(element);
        } else if (forwardedRef) {
          forwardedRef.current = element;
        }
        resize(element);
      },
      [forwardedRef, resize, textareaRef],
    );

    React.useLayoutEffect(() => {
      resize(textareaRef.current);
    }, [resize, textareaRef, value]);

    function handleChange(event: React.ChangeEvent<HTMLTextAreaElement>) {
      resize(event.target);
      setValue(event.target.value);
    }

    function handleKeyDown(event: React.KeyboardEvent<HTMLTextAreaElement>) {
      if (onSubmit && event.key === "Enter" && !event.shiftKey) {
        event.preventDefault();
        onSubmit();
      }
      onKeyDown?.(event);
    }

    return (
      <Textarea
        ref={setRefs}
        className={cn(
          "min-h-[44px] resize-none border-0 bg-transparent p-0 shadow-none focus-visible:ring-0",
          className,
        )}
        disabled={disabled}
        onChange={handleChange}
        onKeyDown={handleKeyDown}
        rows={1}
        value={value}
        {...props}
      />
    );
  },
);
PromptInputTextarea.displayName = "PromptInputTextarea";

type PromptInputActionsProps = React.HTMLAttributes<HTMLDivElement>;

const PromptInputActions = React.forwardRef<HTMLDivElement, PromptInputActionsProps>(
  ({ className, ...props }, ref) => (
    <div ref={ref} className={cn("flex items-center justify-between gap-2", className)} {...props} />
  ),
);
PromptInputActions.displayName = "PromptInputActions";

type PromptInputActionProps = React.HTMLAttributes<HTMLSpanElement> & {
  tooltip: string;
};

const PromptInputAction = React.forwardRef<HTMLSpanElement, PromptInputActionProps>(
  ({ children, className, tooltip, ...props }, ref) => {
    const { disabled } = usePromptInputContext();
    return (
      <span
        ref={ref}
        className={cn(disabled && "pointer-events-none", className)}
        onClick={(event) => event.stopPropagation()}
        title={tooltip}
        {...props}
      >
        {children}
      </span>
    );
  },
);
PromptInputAction.displayName = "PromptInputAction";

export { PromptInput, PromptInputAction, PromptInputActions, PromptInputTextarea };
