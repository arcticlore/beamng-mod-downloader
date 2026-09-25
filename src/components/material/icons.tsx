import type { ReactNode } from "react";

interface IconProps {
  size?: number;
  className?: string;
  title?: string;
}

function Svg({
  size = 24,
  className,
  title,
  children,
  filled = false,
}: IconProps & { children: ReactNode; filled?: boolean }) {
  return (
    <svg
      width={size}
      height={size}
      viewBox="0 0 24 24"
      className={className}
      aria-hidden={title ? undefined : true}
      role={title ? "img" : undefined}
      fill={filled ? "currentColor" : "none"}
      stroke={filled ? "none" : "currentColor"}
      strokeWidth={1.8}
      strokeLinecap="round"
      strokeLinejoin="round"
    >
      {title ? <title>{title}</title> : null}
      {children}
    </svg>
  );
}

export function IconBrowse(p: IconProps) {
  return (
    <Svg {...p}>
      <circle cx="12" cy="12" r="8.5" />
      <path d="M15.5 8.5l-4.7 2.3-2.3 4.7 4.7-2.3 2.3-4.7z" fill="currentColor" stroke="none" />
    </Svg>
  );
}

export function IconDownload(p: IconProps) {
  return (
    <Svg {...p}>
      <path d="M12 3.5v10.5" />
      <path d="M7.5 10l4.5 4.5L16.5 10" />
      <path d="M4 17.5v1a2.5 2.5 0 0 0 2.5 2.5h11a2.5 2.5 0 0 0 2.5-2.5v-1" />
    </Svg>
  );
}

export function IconInstalled(p: IconProps) {
  return (
    <Svg {...p}>
      <path d="M3.5 8.5L12 4l8.5 4.5v7L12 20l-8.5-4.5v-7z" />
      <path d="M3.5 8.5L12 13l8.5-4.5" />
      <path d="M12 13v7" />
    </Svg>
  );
}

export function IconSettings(p: IconProps) {
  return (
    <Svg {...p}>
      <circle cx="12" cy="12" r="3.2" />
      <path d="M12 2.8v2.2M12 19v2.2M4.7 5.2l1.9 1.1M17.4 17.7l1.9 1.1M2.8 12H5M19 12h2.2M4.7 18.8l1.9-1.1M17.4 6.3l1.9-1.1" />
    </Svg>
  );
}

export function IconSearch(p: IconProps) {
  return (
    <Svg {...p}>
      <circle cx="10.8" cy="10.8" r="6.3" />
      <path d="M15.6 15.6L20.5 20.5" />
    </Svg>
  );
}

export function IconClose(p: IconProps) {
  return (
    <Svg {...p}>
      <path d="M5.5 5.5l13 13M18.5 5.5l-13 13" />
    </Svg>
  );
}

export function IconInfo(p: IconProps) {
  return (
    <Svg {...p}>
      <circle cx="12" cy="12" r="8.5" />
      <path d="M12 11v5" />
      <path d="M12 7.8v.4" />
    </Svg>
  );
}

export function IconChevronLeft(p: IconProps) {
  return (
    <Svg {...p}>
      <path d="M14.5 5.5L8 12l6.5 6.5" />
    </Svg>
  );
}

export function IconChevronRight(p: IconProps) {
  return (
    <Svg {...p}>
      <path d="M9.5 5.5L16 12l-6.5 6.5" />
    </Svg>
  );
}

export function IconCancel(p: IconProps) {
  return (
    <Svg {...p}>
      <circle cx="12" cy="12" r="8.5" />
      <path d="M9 9l6 6M15 9l-6 6" />
    </Svg>
  );
}

export function IconDone(p: IconProps) {
  return (
    <Svg {...p}>
      <circle cx="12" cy="12" r="8.5" />
      <path d="M8 12.2l2.7 2.7L16.2 9" />
    </Svg>
  );
}

export function IconError(p: IconProps) {
  return (
    <Svg {...p}>
      <path d="M12 3.8L21 19.5H3L12 3.8z" />
      <path d="M12 10v4" />
      <path d="M12 16.5v.4" />
    </Svg>
  );
}

export function IconExternal(p: IconProps) {
  return (
    <Svg {...p}>
      <path d="M13.5 5.5H18.5V10.5" />
      <path d="M18 6L10.5 13.5" />
      <path d="M18.5 13v3a2 2 0 0 1-2 2h-9a2 2 0 0 1-2-2v-9a2 2 0 0 1 2-2h3" />
    </Svg>
  );
}

export function IconChevronDown(p: IconProps) {
  return (
    <Svg {...p}>
      <path d="M5.5 9L12 15.5 18.5 9" />
    </Svg>
  );
}

export function IconFolder(p: IconProps) {
  return (
    <Svg {...p}>
      <path d="M3.5 7.5A2.5 2.5 0 0 1 6 5h3.2l2 2H18a2.5 2.5 0 0 1 2.5 2.5v7A2.5 2.5 0 0 1 18 19H6a2.5 2.5 0 0 1-2.5-2.5v-9z" />
    </Svg>
  );
}

export function IconRefresh(p: IconProps) {
  return (
    <Svg {...p}>
      <path d="M20 12a8 8 0 1 1-2.34-5.66" />
      <path d="M19.5 3.5v4h-4" />
    </Svg>
  );
}

export function IconShield(p: IconProps) {
  return (
    <Svg {...p}>
      <path d="M12 3l7 2.8v5.4c0 4.4-3 8-7 9.3-4-1.3-7-4.9-7-9.3V5.8L12 3z" />
      <path d="M9 12l2.2 2.2L15.5 9.5" />
    </Svg>
  );
}

export function IconWarning(p: IconProps) {
  return (
    <Svg {...p}>
      <path d="M12 3.8L21 19.5H3L12 3.8z" />
      <path d="M12 10v4" />
      <path d="M12 16.5v.4" />
    </Svg>
  );
}

export function IconUpdate(p: IconProps) {
  return (
    <Svg {...p}>
      <path d="M12 4.5v10" />
      <path d="M7.8 10.5L12 15l4.2-4.5" />
      <path d="M4 18.5v.7a1.8 1.8 0 0 0 1.8 1.8h12.4a1.8 1.8 0 0 0 1.8-1.8v-.7" />
    </Svg>
  );
}

export function IconTrash(p: IconProps) {
  return (
    <Svg {...p}>
      <path d="M4.5 6.5h15" />
      <path d="M9 6.5V5a1.5 1.5 0 0 1 1.5-1.5h3A1.5 1.5 0 0 1 15 5v1.5" />
      <path d="M6.5 6.5l.8 12a1.8 1.8 0 0 0 1.8 1.7h5.8a1.8 1.8 0 0 0 1.8-1.7l.8-12" />
      <path d="M10 10.5v6M14 10.5v6" />
    </Svg>
  );
}

export function IconModPlaceholder(p: IconProps) {
  return (
    <Svg {...p}>
      <path d="M4 6.5A2.5 2.5 0 0 1 6.5 4h4l2 2h5A2.5 2.5 0 0 1 20 8.5v9a2.5 2.5 0 0 1-2.5 2.5h-11A2.5 2.5 0 0 1 4 17.5v-11z" />
      <path d="M8 12l2.5 2.5L15.5 9.5" />
    </Svg>
  );
}

export function IconMore(p: IconProps) {
  return (
    <Svg {...p}>
      <circle cx="12" cy="5.5" r="1.2" fill="currentColor" stroke="none" />
      <circle cx="12" cy="12" r="1.2" fill="currentColor" stroke="none" />
      <circle cx="12" cy="18.5" r="1.2" fill="currentColor" stroke="none" />
    </Svg>
  );
}

export function IconArrowLeft(p: IconProps) {
  return (
    <Svg {...p}>
      <path d="M19 12H5" />
      <path d="M10.5 6.5L5 12l5.5 5.5" />
    </Svg>
  );
}