/**
 * Iconfamilie - Kapitel 2.16
 * Outline, geometrisch, 1.5-2 px Stroke, klare Silhouetten, reduzierte Details.
 * Der Telefonhoerer erscheint nur als Funktionsicon (Call), nie als Markensymbol.
 */
import type { JSX } from "react";

export type IconName =
  | "user"
  | "extension"
  | "device"
  | "number"
  | "route"
  | "trunk"
  | "queue"
  | "group"
  | "gateway"
  | "flow"
  | "provisioning"
  | "security"
  | "network"
  | "call"
  | "voicemail"
  | "settings"
  | "dashboard"
  | "message"
  | "logs"
  | "update"
  | "plus"
  | "close"
  | "check"
  | "warning"
  | "search"
  | "logout"
  | "chevron";

interface IconProps {
  name: IconName;
  size?: number;
  className?: string;
  title?: string;
}

const paths: Record<IconName, JSX.Element> = {
  user: (
    <>
      <circle cx="12" cy="8" r="3.5" />
      <path d="M4.5 20c0-3.6 3.4-6 7.5-6s7.5 2.4 7.5 6" />
    </>
  ),
  extension: (
    <>
      <rect x="4" y="3.5" width="16" height="17" rx="2.5" />
      <path d="M8 8h8M8 12h8M8 16h4" />
    </>
  ),
  device: (
    <>
      <rect x="5" y="3" width="14" height="18" rx="2.5" />
      <path d="M9.5 18.5h5" />
      <path d="M8 7h8v4H8z" />
    </>
  ),
  number: (
    <>
      <path d="M9 3.5 7 20.5M17 3.5l-2 17" />
      <path d="M4 9h16M3.5 15h16" />
    </>
  ),
  route: (
    <>
      <circle cx="6" cy="6" r="2.5" />
      <circle cx="18" cy="18" r="2.5" />
      <path d="M6 8.5v6a3 3 0 0 0 3 3h6.5" />
    </>
  ),
  trunk: (
    <>
      <rect x="3" y="9" width="6" height="6" rx="1.5" />
      <rect x="15" y="9" width="6" height="6" rx="1.5" />
      <path d="M9 12h6" />
    </>
  ),
  queue: (
    <>
      <rect x="3.5" y="5" width="17" height="4" rx="1.5" />
      <rect x="3.5" y="11" width="17" height="4" rx="1.5" />
      <path d="M7 19h10" />
    </>
  ),
  group: (
    <>
      <circle cx="9" cy="8.5" r="3" />
      <path d="M3.5 19c0-3 2.5-5 5.5-5s5.5 2 5.5 5" />
      <path d="M16 6.2a3 3 0 0 1 0 5.6M17 14.5c2 .8 3.5 2.4 3.5 4.5" />
    </>
  ),
  gateway: (
    <>
      <rect x="3.5" y="13" width="17" height="7" rx="2" />
      <path d="M7 16.5h.01M11 16.5h.01" />
      <path d="M12 10V4M8.5 7 12 3.5 15.5 7" />
    </>
  ),
  flow: (
    <>
      <rect x="3" y="3.5" width="7" height="6" rx="1.5" />
      <rect x="14" y="14.5" width="7" height="6" rx="1.5" />
      <path d="M6.5 9.5v5a3 3 0 0 0 3 3H14" />
    </>
  ),
  provisioning: (
    <>
      <path d="M12 3.5v9" />
      <path d="M8 6.5 12 2.8l4 3.7" />
      <rect x="3.5" y="14" width="17" height="6.5" rx="2" />
      <path d="M7 17.2h.01" />
    </>
  ),
  security: (
    <>
      <path d="M12 3.2 5 6v6c0 4.2 2.9 7.6 7 8.8 4.1-1.2 7-4.6 7-8.8V6z" />
      <path d="m9 12 2 2 4-4" />
    </>
  ),
  network: (
    <>
      <circle cx="12" cy="12" r="8.5" />
      <path d="M3.5 12h17M12 3.5c2.2 2.4 3.4 5.4 3.4 8.5s-1.2 6.1-3.4 8.5c-2.2-2.4-3.4-5.4-3.4-8.5S9.8 5.9 12 3.5z" />
    </>
  ),
  call: (
    <path d="M7.2 4h-2A1.8 1.8 0 0 0 3.5 6c.4 7.5 6.5 13.6 14 14a1.8 1.8 0 0 0 2-1.8v-2a1.5 1.5 0 0 0-1.2-1.5l-2.6-.5a1.5 1.5 0 0 0-1.5.6l-.9 1.2a12.5 12.5 0 0 1-5.4-5.4l1.2-.9a1.5 1.5 0 0 0 .6-1.5l-.5-2.6A1.5 1.5 0 0 0 7.2 4z" />
  ),
  voicemail: (
    <>
      <circle cx="6.5" cy="13" r="3.8" />
      <circle cx="17.5" cy="13" r="3.8" />
      <path d="M6.5 16.8h11" />
    </>
  ),
  settings: (
    <>
      <circle cx="12" cy="12" r="3" />
      <path d="M12 2.8v2.4M12 18.8v2.4M4.5 12H2.1M21.9 12h-2.4M6.7 6.7 5 5M19 19l-1.7-1.7M6.7 17.3 5 19M19 5l-1.7 1.7" />
    </>
  ),
  dashboard: (
    <>
      <rect x="3.5" y="3.5" width="7" height="7" rx="1.5" />
      <rect x="13.5" y="3.5" width="7" height="7" rx="1.5" />
      <rect x="3.5" y="13.5" width="7" height="7" rx="1.5" />
      <rect x="13.5" y="13.5" width="7" height="7" rx="1.5" />
    </>
  ),
  message: (
    <path d="M4 5.5h16v11H9.5L5.5 20v-3.5H4z" />
  ),
  logs: (
    <>
      <path d="M5 4.5h14v15H5z" />
      <path d="M8.5 9h7M8.5 12.5h7M8.5 16h4" />
    </>
  ),
  update: (
    <>
      <path d="M20 12a8 8 0 1 1-2.6-5.9" />
      <path d="M20.5 4v4.5H16" />
    </>
  ),
  plus: <path d="M12 5v14M5 12h14" />,
  close: <path d="M6 6l12 12M18 6L6 18" />,
  check: <path d="m5 12.5 4.5 4.5L19 7.5" />,
  warning: (
    <>
      <path d="M12 4 2.8 20h18.4z" />
      <path d="M12 10v4M12 17h.01" />
    </>
  ),
  search: (
    <>
      <circle cx="11" cy="11" r="6.5" />
      <path d="m16 16 4.5 4.5" />
    </>
  ),
  logout: (
    <>
      <path d="M14 4.5H6a1.5 1.5 0 0 0-1.5 1.5v12A1.5 1.5 0 0 0 6 19.5h8" />
      <path d="M17 8.5 20.5 12 17 15.5M20 12H10" />
    </>
  ),
  chevron: <path d="m9 5.5 6.5 6.5L9 18.5" />,
};

export function Icon({ name, size = 18, className, title }: IconProps) {
  return (
    <svg
      className={className ? `icon ${className}` : "icon"}
      width={size}
      height={size}
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      strokeWidth={1.7}
      strokeLinecap="round"
      strokeLinejoin="round"
      role={title ? "img" : "presentation"}
      aria-hidden={title ? undefined : true}
      focusable="false"
    >
      {title ? <title>{title}</title> : null}
      {paths[name]}
    </svg>
  );
}
