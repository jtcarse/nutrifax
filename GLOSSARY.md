# Nutrifax

Language for turning restaurant nutrition sheets into foods that can be selected and exported.

## Language

**Nutrition sheet**:
A restaurant PDF that publishes nutrition values for multiple menu items, often as a table.

**Menu item**:
A food or drink with its own published nutrition values on a nutrition sheet. A size or preparation variant is a separate menu item when the sheet gives it a separate nutrition entry.

**Serving**:
The amount of a menu item to which its published nutrition values apply. A nutrition sheet may specify a weight, volume, count, or only an unnamed serving.

**Meal selection**:
The subset of menu items a person chooses from an imported nutrition sheet for export.

**Nutrition label**:
A conventional nutrition facts image generated from one selected menu item's published values.

**Flagged value**:
A value from a nutrition sheet that needs a person's decision before it can be used on a nutrition label, such as a range, an inequality, or an unclear unit.
