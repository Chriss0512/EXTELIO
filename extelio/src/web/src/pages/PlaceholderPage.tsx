/**
 * Platzhalter fuer Bereiche, die in der Spezifikation vorgesehen sind, deren
 * Funktion aber den Telefonie-Core voraussetzt. Der Bereich wird ehrlich als
 * noch nicht verfuegbar ausgewiesen statt mit Beispieldaten gefuellt.
 */
import { EmptyState, PageHeader } from "../components/ui";
import type { IconName } from "../design/icons";

export function PlaceholderPage({
  title,
  subtitle,
  icon,
  text,
}: {
  title: string;
  subtitle: string;
  icon: IconName;
  text: string;
}) {
  return (
    <>
      <PageHeader title={title} subtitle={subtitle} />
      <EmptyState icon={icon} title="Noch nicht verfügbar" text={text} />
    </>
  );
}
