#pragma once

#include "../UIElement.h"

#include "../Components/Slider.h"

#include "../../Character/Inventory/MakerRules.h"
#include "../../Graphics/Text.h"
#include "../../Graphics/Texture.h"

#include "../../Audio/Audio.h"

namespace ms
{
	// The Item Maker's window (v83 UIWnd type 0x17 on UIWindow.img/Maker, reports c006 and c006b), opened by casting
	// the Maker skill. A category and an item are picked from two drop lists; the recipe's ingredients show with how
	// many of each the player has, and the fee. Start asks first, fills the gauge, then sends ITEM_MAKE, and the
	// window waits for MAKER_RESULT. For a monster crystal, the drops are dragged in. It is dragged by its top.
	class UIMaker : public UIElement
	{
	public:
		static constexpr Type TYPE = UIElement::Type::MAKER;
		static constexpr bool FOCUSED = false;
		static constexpr bool TOGGLED = false;

		UIMaker();

		~UIMaker() override;

		void draw(float alpha) const override;

		void update() override;

		void remove_cursor() override;

		Cursor::State send_cursor(bool clicked, Point<int16_t> cursorpos) override;

		void send_scroll(double yoffset) override;

		void send_key(int32_t keycode, bool pressed, bool escape) override;

		bool send_icon(const Icon &icon, Point<int16_t> cursorpos) override;

		UIElement::Type get_type() const override;

		// MAKER_RESULT's code, and for one that made something its count and name, or what it took apart
		// (FUN_00827264).
		void answer(int32_t result, int32_t count, const std::string &name, bool disassembled);

		// The recipes in ItemMake.img, by group.
		static const std::vector<maker::Recipe> &recipes();

	protected:
		Button::State button_pressed(uint16_t buttonid) override;

	private:
		// v83's generic drop list (CCtrlComboBox, report c006d): a box of Basic.img/ComboBox4's three pieces, and when
		// open a grey list of up to ten rows below it with a scroll bar past that.
		class DropList
		{
		public:
			struct Choice
			{
				std::string text;
				int32_t data;
			};

			void init(Point<int16_t> position, int16_t width);

			void set(std::vector<Choice> choices);

			void draw(Point<int16_t> parent) const;

			// Under the cursor, from the window's top left: the box, or an open list.
			bool in_box(Point<int16_t> cursor) const;

			bool in_list(Point<int16_t> cursor) const;

			// The row under the cursor in an open list, or -1.
			int32_t row_at(Point<int16_t> cursor) const;

			// Moves the cursor over the box or an open list; true when the list's scroll bar takes it.
			bool hover(Point<int16_t> cursor, bool clicked);

			// The cursor has left the window.
			void leave();

			void scroll(int32_t rows);

			void toggle();

			void close();

			void select(size_t index);

			bool is_open() const;

			size_t selected() const;

			int32_t data() const;

			void enable(bool on);

		private:
			size_t shown_rows() const;

			int16_t list_height() const;

			int16_t row_width() const;

			Texture pieces[Button::State::NUM_STATES][3];
			Point<int16_t> position;
			int16_t width = 0;
			std::vector<Choice> choices;
			std::vector<Text> rows;
			std::vector<Text> lit_rows;
			Text shown;
			Slider slider;
			size_t chosen = 0;
			size_t first = 0;
			int32_t hovered = -1;
			bool over_box = false;
			bool open = false;
			bool enabled = true;
		};

		// One ingredient: the item, its icon and how many of it there are and the recipe needs.
		struct Slot
		{
			int32_t item_id = 0;
			int32_t owned = 0;
			int32_t needed = 0;
		};

		enum Buttons : uint16_t
		{
			START,
			CANCEL,
			CLOSE
		};

		// FUN_00824a06's states: nothing picked, ready, the gauge filling, and waiting on the server.
		enum class State
		{
			IDLE,
			READY,
			FILLING,
			WAITING
		};

		void pick_category();

		void pick_item();

		// Puts the recipe for `item_id` in, mode 1 (FUN_00823e17).
		void show_recipe(int32_t item_id);

		// Puts the monster crystal in, mode 3 (FUN_0082411b).
		void show_crystal();

		// Takes a gem, a stimulator or an equipment to take apart dropped from the inventory (FUN_0082637e,
		// FUN_008265a1, FUN_00825dd2).
		void drop_crystal(int32_t id);

		void drop_gem(int32_t id);

		void drop_stimulator(int32_t id);

		void drop_equip(int32_t id, int16_t slot);

		// Puts an item in the random make, as many as the player says (FUN_00825dd2, mode 2).
		void drop_random(int32_t id);
		// Fills the first empty slot once the player has said how many.
		void apply_random(int32_t id, int32_t quantity);

		// What the fee is now, for a recipe with what has been put in or a disassembly.
		int32_t current_fee() const;

		// Whether the tab what it makes goes to has room (FUN_0082744f): one free slot, or for a disassembly one for
		// each ingredient of the equip's recipe. With `tell` the player is told when not.
		bool has_room(bool tell) const;

		// The gem slots there are now.
		int32_t gem_slots() const;

		// Counts the ingredients again and enables Start when they are all there.
		void recount();

		bool can_start() const;

		void start();

		void confirm();

		void send();

		void close();

		void clear_tooltip();

		void show_tooltip(Point<int16_t> cursor);

		// Which ingredient slot the cursor is on, or -1.
		int32_t slot_at(Point<int16_t> cursor) const;

		static int32_t crystal_of(int32_t item_id);

		maker::Maker facts;
		std::vector<const maker::Recipe *> listed;
		DropList category;
		DropList item;
		// The mode of what is picked: 0 for nothing, 1 a recipe, 2 a random make, 3 a crystal, 4 a disassembly.
		int32_t mode;
		const maker::Recipe *recipe;
		std::vector<Slot> slots;
		// The gems put in, in their slots' order, and whether the stimulator is, and which: the recipe's, or for the
		// random make any.
		std::vector<int32_t> gems;
		bool stimulated;
		int32_t stimulator;
		// The Equip tab's slot of the equipment to take apart.
		int16_t equip_slot;
		State state;
		int32_t elapsed;
		Texture not_use;
		Texture random_make;
		Texture random_recipe;
		Texture bar;
		Texture gauge;
		Text count;
		Text fee;
		// Sound/UI.img/ItemMake, looped while the gauge fills.
		Sound working;
		bool was_clicked;
		bool dragged;
		Point<int16_t> drag_from;
	};
}
