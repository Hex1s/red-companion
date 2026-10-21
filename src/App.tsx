import { useState } from "react";
import { useTranslation } from "react-i18next";
import { CharacterCard, type CharacterCardData } from "@/components/character/character-card";
import { AppShell } from "@/components/layout/app-shell";
import { Button } from "@/components/ui/button";
import { Tabs, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { SUPPORTED_LANGUAGES } from "@/i18n";

// Placeholder data for checking the design; will be replaced by data from the Rust core.
const DEMO: CharacterCardData[] = [
  { id: "1", name: "Джонни", initiative: 18, hp: 24, maxHp: 40, spHead: 11, spBody: 7 },
  { id: "2", name: "Ви", initiative: 15, hp: 40, maxHp: 40, spHead: 7, spBody: 11 },
  { id: "3", name: "Бустер", initiative: 12, hp: 0, maxHp: 20, spHead: 4, spBody: 4 },
];

export default function App() {
  const { t, i18n } = useTranslation();
  const [selected, setSelected] = useState(DEMO[0].id);
  const character = DEMO.find((c) => c.id === selected) ?? DEMO[0];

  return (
    <AppShell
      header={
        <>
          <h1 className="text-lg font-bold text-primary">{t("app.title")}</h1>
          <Tabs defaultValue="combat" className="flex-1">
            <TabsList>
              <TabsTrigger value="combat">{t("nav.combat")}</TabsTrigger>
              <TabsTrigger value="characters">{t("nav.characters")}</TabsTrigger>
              <TabsTrigger value="rules">{t("nav.rules")}</TabsTrigger>
            </TabsList>
          </Tabs>
          <div className="flex gap-1">
            {SUPPORTED_LANGUAGES.map((lng) => (
              <Button
                key={lng}
                size="xs"
                variant={i18n.resolvedLanguage === lng ? "default" : "outline"}
                onClick={() => i18n.changeLanguage(lng)}
              >
                {lng}
              </Button>
            ))}
          </div>
        </>
      }
      main={
        <article className="max-w-3xl space-y-4">
          <h2 className="text-2xl font-bold">{character.name}</h2>
          <p className="text-muted-foreground">{t("app.subtitle")}</p>
          <div className="num grid grid-cols-5 gap-2">
            {(["INT", "REF", "DEX", "TECH", "COOL"] as const).map((stat) => (
              <div key={stat} className="cut [--cut:6px] p-2 text-center">
                <div className="text-xs text-muted-foreground">{t(`stats.${stat}`)}</div>
                <div className="text-xl font-bold">6</div>
              </div>
            ))}
          </div>
        </article>
      }
      rail={DEMO.map((c) => (
        <CharacterCard
          key={c.id}
          character={c}
          active={c.id === DEMO[0].id}
          selected={c.id === selected}
          onSelect={() => setSelected(c.id)}
        />
      ))}
    />
  );
}
