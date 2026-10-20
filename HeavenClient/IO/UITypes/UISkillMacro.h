#pragma once

#include "../UIElement.h"

#include "SkillMacroLayout.h"

#include "../Components/Icon.h"
#include "../Components/Slider.h"
#include "../Components/Textfield.h"

#include <array>

namespace ms
{
	// The v83 skill macro window (UIClassic.nx UIWindow.img/SkillMacro, report
	// 206 4), which the skill window's Macro button shows at its right. It
	// lists the five macros three at a time: skills are dragged into their
	// slots from the skill window, and the selected macro is named and set to
	// shout below.
	class UISkillMacro : public UIElement
	{
	public:
		static constexpr Type TYPE = UIElement::Type::SKILLMACRO;
		static constexpr bool FOCUSED = false;
		static constexpr bool TOGGLED = true;

		// Shown at the right of the skill window at `skillbook`.
		UISkillMacro(Point<int16_t> skillbook);

		void draw(float alpha) const override;

		void update() override;

		void toggle_active() override;

		void remove_cursor() override;

		Cursor::State send_cursor(bool clicked, Point<int16_t> cursorpos) override;

		void send_scroll(double yoffset) override;

		bool send_icon(const Icon &icon, Point<int16_t> cursorpos) override;

		UIElement::Type get_type() const override;

		// Follows the skill window at `skillbook`.
		void dock(Point<int16_t> skillbook);

		// Hides the window and forgets the selection, as v83 does in closing
		// the skill window that owns it.
		void close();

	protected:
		Button::State button_pressed(uint16_t buttonid) override;

	private:
		// A skill taken out of a macro's slot. On the field it leaves the slot
		// empty; v83 binds it to no key.
		class SlotIcon : public Icon::Type
		{
		public:
			SlotIcon(int32_t skill_id);

			void drop_on_stage() const override;
			void drop_on_equips(EquipSlot::Id) const override {}
			bool drop_on_items(InventoryType::Id, EquipSlot::Id, int16_t, bool) const override { return true; }
			void drop_on_bindings(Point<int16_t>, bool) const override {}
			void set_count(int16_t) override {}
			Icon::IconType get_type() override;

		private:
			int32_t skill_id;
		};

		// A macro itself, dragged from its icon to a key in the key config or
		// a quick slot.
		class MacroIcon : public Icon::Type
		{
		public:
			MacroIcon(size_t index);

			void drop_on_stage() const override {}
			void drop_on_equips(EquipSlot::Id) const override {}
			bool drop_on_items(InventoryType::Id, EquipSlot::Id, int16_t, bool) const override { return true; }
			void drop_on_bindings(Point<int16_t> cursorposition, bool remove) const override;
			Keyboard::Mapping get_mapping() const override;
			void set_count(int16_t) override {}
			Icon::IconType get_type() override;

		private:
			size_t index;
		};

		using Hit = skill_macro_layout::Hit;

		// v83 FUN_008a984d.
		void select(size_t index);

		void start_drag(Icon icon, Point<int16_t> offset);

		// Takes a skill dropped on the window or the field (v83 FUN_004fb112).
		// A skill from a slot leaves it, and goes in the slot under `cursor`
		// if there is one.
		void drop_skill(int32_t skill_id, bool from_slot, bool on_field, Point<int16_t> cursor);

		void save();

		// Raises a notice, giving up the field's keys for it.
		void notice(const char *text);

		void show_tooltip(const Hit &hit);

		void clear_tooltip();

		// Lets go of the drag and the field's keys.
		void release();

		enum Buttons : uint16_t
		{
			BT_OK
		};

		static constexpr size_t ROWS = skill_macro_layout::ROWS;

		Texture highlight;
		Texture slot;
		Texture icon_slot;
		Texture top_line;
		Texture bottom_line;
		Texture name_panel;
		std::array<Texture, 5> macro_icons;
		std::array<Texture, 2> checkbox;

		Slider slider;
		Textfield field;

		size_t scroll;
		// The selected macro, or -1.
		int16_t selected;
		bool shout;
		bool held;
		bool checking;

		Icon dragged;
		// The slot `dragged` came from.
		size_t drag_index;
		size_t drag_slot;
	};
}
