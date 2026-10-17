import type { Section } from "./AppSidebar";

type TopBarProps = {
  activeSection: Section;
  search: string;
  onSearchChange: (value: string) => void;
};

export function TopBar({ activeSection, search, onSearchChange }: TopBarProps) {
  return (
    <header className="topbar">
      <span className="section-label">{activeSection}</span>
      <label className="search-field">
        <span className="sr-only">Search your library</span>
        <svg aria-hidden="true" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.5"><circle cx="10.5" cy="10.5" r="6.5" /><path d="m16 16 5 5" /></svg>
        <input type="search" placeholder="Search the collection" value={search} onChange={(event) => onSearchChange(event.currentTarget.value)} />
      </label>
    </header>
  );
}
