#pragma once

#include <array>
#include <cstdint>
#include <vector>

#include "../UIDragElement.h"

#include "../Components/ClassicTabs.h"
#include "../Components/Slider.h"
#include "../Graphics/Text.h"

#include "../Character/Inventory/Inventory.h"

#include "../Net/Handlers/StorageContents.h"

namespace ms
{
	// The storage an NPC keeps for the account (v83 CTrunkDlg): its items on
	// the left, the player's storable ones on the right, and one tab choosing
	// the bag both show.
	class UIStorage : public UIDragElement<PosSTORAGE>
	{
	public:
		static constexpr Type TYPE = UIElement::Type::STORAGE;
		static constexpr bool FOCUSED = true;
		static constexpr bool TOGGLED = true;

		UIStorage(const Inventory &inventory);

		void draw(float alpha) const override;

		void update() override;

		void remove_cursor() override;

		Cursor::State send_cursor(bool clicked, Point<int16_t> position) override;
		void doubleclick(Point<int16_t> cursorpos) override;

		void send_scroll(double yoffset) override;

		void send_key(int32_t keycode, bool pressed, bool escape) override;

		UIElement::Type get_type() const override;

		// Opens npcid's storage with everything in it.
		void open(int32_t npcid, const storage_contents::Contents &contents);

		// A request was answered with what changed.
		void change(const storage_contents::Contents &contents);

		// A request was answered, or refused.
		void answered();

		// The player's bag of a type changed.
		void modify(InventoryType::Id type);

	protected:
		Button::State button_pressed(uint16_t buttonid) override;

	private:
		struct Row
		{
			int32_t itemid;
			int16_t count;
			// The inventory slot of a carried item.
			int16_t slot;
			Texture icon;
			// The name, wrapped onto two lines.
			Text name[2];
		};

		struct List
		{
			std::vector<Row> rows;
			int16_t offset = 0;
			int16_t selection = -1;
		};

		void change_tab(InventoryType::Id tab);

		void refresh_stored();

		void refresh_carried();

		void draw_list(const List &list, Point<int16_t> origin, int16_t rows, int16_t select_x, int16_t limit) const;

		// The row of a list under the cursor, or -1.
		static int16_t row_at(Point<int16_t> cursor, Point<int16_t> origin, int16_t rows);

		void show_item(Point<int16_t> cursor);

		void clear_tooltip();

		bool can_request() const;

		void take_out();

		void store();

		void take_meso();

		void save_meso();

		void close();

		static Row make_row(int32_t itemid, int16_t count, int16_t slot);

		static constexpr int16_t STORED_ROWS = 5;
		static constexpr int16_t CARRIED_ROWS = 4;

		enum Buttons : uint16_t
		{
			EXIT,
			GET,
			PUT,
			SORT,
			OUT_COIN,
			IN_COIN,
			TAB0,
			TAB4 = TAB0 + 4,
			STORED0,
			STORED4 = STORED0 + STORED_ROWS - 1,
			CARRIED0,
			CARRIED3 = CARRIED0 + CARRIED_ROWS - 1,
			NUM_BUTTONS
		};

		const Inventory &inventory;

		int32_t npcid;
		Texture npc;
		Texture enabled_row;
		Texture selected_row;
		ClassicTabs tabs;
		Slider stored_slider;
		Slider carried_slider;
		Text stored_meso_label;
		Text carried_meso_label;

		// The trunkPut and trunkGet fees of the NPC.
		int32_t put_fee;
		int32_t get_fee;

		uint8_t slots;
		int32_t meso;
		// By inventory type, as the server lists them.
		std::array<std::vector<storage_contents::Stored>, 6> bags;

		InventoryType::Id tab;
		List stored;
		List carried;

		// A request is out, and the next waits for its answer.
		bool pending;

		Point<int16_t> lastcursorpos;
	};
}
