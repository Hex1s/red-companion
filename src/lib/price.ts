import type { TFunction } from "i18next";
import type { PriceCategory } from "@/bindings/PriceCategory";

/**
 * The category a price falls into: the highest one whose minimum it reaches (the table comes from the
 * Rust core, `price_category_of` there does the same). Empty before the table is loaded.
 */
export function priceCategoryId(categories: PriceCategory[], price: number): string | null {
  const found = [...categories].reverse().find((c) => price >= c.min_price) ?? categories[0];
  return found?.id ?? null;
}

/** Name of the price's category in the interface language, `""` while the table is not loaded. */
export function priceCategoryName(categories: PriceCategory[], price: number, t: TFunction): string {
  const id = priceCategoryId(categories, price);
  return id ? t(`price.category.${id}`) : "";
}
