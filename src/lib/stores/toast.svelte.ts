export interface Toast {
  id: number;
  message: string;
  type: "error" | "success" | "info";
}

let toasts = $state<Toast[]>([]);
let nextId = 0;

export function getToasts(): Toast[] {
  return toasts;
}

export function showToast(
  message: string,
  type: "error" | "success" | "info" = "info"
): void {
  const id = nextId++;
  toasts = [...toasts, { id, message, type }];

  // Auto-dismiss after 5 seconds
  setTimeout(() => {
    dismissToast(id);
  }, 5000);
}

export function showError(message: string): void {
  showToast(message, "error");
}

export function showSuccess(message: string): void {
  showToast(message, "success");
}

export function dismissToast(id: number): void {
  toasts = toasts.filter((t) => t.id !== id);
}
