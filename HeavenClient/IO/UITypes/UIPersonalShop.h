#pragma once

#include "../UIDragElement.h"

#include "../Components/Icon.h"
#include "../Components/Slider.h"
#include "../Components/Textfield.h"

#include "../../Character/Look/CharLook.h"
#include "../../Graphics/Text.h"
#include "../../Graphics/Texture.h"
#include "../../Net/Handlers/PersonalShop.h"
#include "../../Template/Rectangle.h"

#include <array>
#include <chrono>
#include <vector>

namespace ms
{
	// A personal shop (v83 CPersonalShopDlg, FUN_006fa0f5): the owner and up
	// to three visitors along the top, the items for sale on the left and a
	// chat on the right. The owner fills it before opening; visitors buy.
	class UIPersonalShop : public UIDragElement<PosPERSONALSHOP>
	{
	public:
		static constexpr Type TYPE = UIElement::Type::PERSONALSHOP;
		// Not focused, so the owner can drag items in from the inventory.
		static constexpr bool FOCUSED = false;
		static constexpr bool TOGGLED = false;

		UIPersonalShop(const mini_room::Enter &enter);

		void draw(float inter) const override;

		void update() override;

		void remove_cursor() override;

		Cursor::State send_cursor(bool clicked, Point<int16_t> cursorpos) override;

		bool send_icon(const Icon &icon, Point<int16_t> cursorpos) override;

		void doubleclick(Point<int16_t> cursorpos) override;

		void send_scroll(double yoffset) override;

		void send_key(int32_t keycode, bool pressed, bool escape) override;

		// The empty lower right corner lets clicks through (FUN_006fc3a8).
		bool is_in_range(Point<int16_t> cursorpos) const override;

		UIElement::Type get_type() const override;

		// Someone came in to look.
		void visit(const mini_room::Member &member);

		// The visitor in `slot` left.
		void seat_left(uint8_t slot);

		// A member's equipment changed.
		void change_look(uint8_t slot, const LookEntry &look);

		// A line the member in `slot` said, "name : text", in the colour of
		// their name tag; others' words are filtered.
		void add_chat(uint8_t slot, const std::string &line);

		// The items for sale, after any change (mode 0x19).
		void set_listings(const std::vector<mini_room::Listing> &listings);

		// A hired merchant's window rather than a shop's.
		bool is_merchant() const;

		// The mesos a merchant holds, before its rows in mode 0x19.
		void set_merchant_meso(int32_t meso);

		// The merchant crossed off what sold and has `meso` (mode 0x28).
		void organized(int32_t meso);

		// The owner closed the merchant; leaves, saying what came back
		// (mode 0x2a).
		void store_closed(int8_t code);

		// The owner took the merchant's mesos (mode 0x2c).
		void meso_taken();

		// The owner's list `list` (MERCHANTVISITORS or MERCHANTBLACKLIST)
		// opened or closed: its button is off while it shows.
		void list_shown(UIElement::Type list, bool shown);

		// Someone bought from my shop (mode 0x1a).
		void sold(const personal_shop::SoldNotice &notice);

		// Why my purchase failed (mode 0x18).
		void buy_result(int8_t result);

		// The shop closed on us; closes the window, saying why.
		void leave(int8_t reason);

		// Closes the window without telling the server.
		void close();

		// Asks how many of an inventory item to sell and for how much.
		void sell(InventoryType::Id tab, int16_t slot, int32_t itemid, int16_t count);

		// Takes row `index` back into the inventory before the shop opens.
		void take_back(int16_t index) const;

		const personal_shop::Sales &get_sales() const;

		uint8_t get_own_slot() const;

		// Sets up a shop, or a hired merchant, with the permit in cash slot
		// `slot` (FUN_00a1dd99): one room at a time, in the free market.
		static void create(int16_t slot, int32_t itemid, bool merchant = false);

		// Asks the server whether the hired merchant permit in cash slot
		// `slot` may be used (0xa27b7d); it answers with recv 50.
		static void hire(int16_t slot, int32_t itemid, int64_t cash_id);

		// The server's yes: the permit asked about sets up the merchant.
		static void hire_approved();

	protected:
		Button::State button_pressed(uint16_t buttonid) override;

	private:
		bool is_owner() const;

		int16_t row_at(Point<int16_t> cursor) const;

		void buy(int16_t index);

		void ban(uint8_t slot);

		void show_owner_info() const;

		void submit(const std::string &text);

		void add_line(const std::string &line, Color::Name color);

		void scroll_to_bottom();

		void refresh_rows();

		void clear_tooltip();

		void exit();

		// My shop has asked to open: a personal shop is shown open, a
		// merchant left to it. `sync_blacklist` sends the Blacklist's
		// changes while a personal shop is open.
		void opened(bool sync_blacklist);

		// How a hired merchant's window closes (0x51809d): `reason` 1
		// leaves, 2 also reopens a merchant in maintenance, 8 closes it.
		void close_merchant(int8_t reason);

		// A row of my shop dragged back to the inventory (FUN_006fb368).
		class RowIcon : public Icon::Type
		{
		public:
			RowIcon(int16_t index, int32_t itemid);

			void drop_on_stage() const override {}
			void drop_on_equips(EquipSlot::Id) const override {}
			bool drop_on_items(InventoryType::Id, EquipSlot::Id, int16_t, bool) const override { return true; }
			void drop_on_bindings(Point<int16_t>, bool) const override {}
			InventoryType::Id get_return_tab() const override;
			void drop_on_inventory() const override;
			void set_count(int16_t) override {}
			Icon::IconType get_type() override { return Icon::IconType::ITEM; }

		private:
			int16_t index;
			int32_t itemid;
		};

		enum Buttons : uint16_t
		{
			START,
			BUY,
			CLOSE,
			EXIT,
			INFO,
			ITEM,
			CLAIM,
			ENTER,
			BAN1,
			BAN2,
			BAN3,
			// A hired merchant's owner's (0x6fa4b2)
			RESUME,
			ARRANGE,
			COIN,
			BLACKLIST,
			VISITORS
		};

		struct Member
		{
			bool present = false;
			CharLook look;
			Text name;
		};

		struct Row
		{
			Text name;
			Text price;
		};

		Texture backgrnd;
		Texture select;

		std::array<Member, personal_shop::SEATS> members;
		uint8_t own_slot;
		bool merchant;
		Texture employee;
		int32_t merchant_meso;
		// How long the merchant had been open when we came in, and when
		// that was, for the time it has left.
		int64_t open_ms;
		std::chrono::steady_clock::time_point entered;
		Text countdown;
		// Rows the shop holds (16).
		uint8_t max_items;
		// Visitors only ever see an open shop.
		bool open;

		Text title;
		Text meso;

		std::vector<mini_room::Listing> listings;
		std::vector<Row> rows;
		int16_t selected;
		int16_t first_row;
		Slider items;
		Icon dragged_row;
		// Where the cursor last was, to send the wheel to the right list.
		Point<int16_t> last_cursor;

		personal_shop::Sales sales;

		struct Line
		{
			Text text;
			int16_t rows;
		};

		std::vector<Line> log;
		int16_t log_rows;
		int16_t log_row;
		Slider chat;
		Textfield input;
	};

	// The owner's dialog for putting an item up (v83 FUN_006fe151): how many,
	// at what price, and optionally in bundles.
	class UIShopSell : public UIElement
	{
	public:
		static constexpr Type TYPE = UIElement::Type::SHOPSELL;
		static constexpr bool FOCUSED = true;
		static constexpr bool TOGGLED = false;

		UIShopSell(InventoryType::Id tab, int16_t slot, int32_t itemid, int16_t quantity, bool whole, bool cash, bool merchant);

		void draw(float inter) const override;

		void update() override;

		Cursor::State send_cursor(bool clicked, Point<int16_t> cursorpos) override;

		void send_key(int32_t keycode, bool pressed, bool escape) override;

		UIElement::Type get_type() const override;

	protected:
		Button::State button_pressed(uint16_t buttonid) override;

	private:
		void confirm();

		void cancel();

		Textfield &box(personal_shop::Box box);

		enum Buttons : uint16_t
		{
			OK,
			CANCEL
		};

		Texture backgrnd;
		Texture bar;
		std::array<Texture, 2> checkboxes;

		InventoryType::Id tab;
		int16_t slot;
		int32_t itemid;
		int16_t quantity;
		bool merchant;
		bool whole;
		bool cash;
		bool bundled;

		Textfield count;
		Textfield price;
		Textfield per_bundle;
	};

	// What the owner sold since opening, with the totals (v83 FUN_006fed4a).
	class UIShopSales : public UIElement
	{
	public:
		static constexpr Type TYPE = UIElement::Type::SHOPSALES;
		static constexpr bool FOCUSED = false;
		static constexpr bool TOGGLED = false;

		UIShopSales();

		void draw(float inter) const override;

		void remove_cursor() override;

		Cursor::State send_cursor(bool clicked, Point<int16_t> cursorpos) override;

		void send_scroll(double yoffset) override;

		void send_key(int32_t keycode, bool pressed, bool escape) override;

		UIElement::Type get_type() const override;

		// A sale came in.
		void refresh();

	protected:
		Button::State button_pressed(uint16_t buttonid) override;

	private:
		int16_t total_rows() const;

		enum Buttons : uint16_t
		{
			CLOSE
		};

		struct Record
		{
			int32_t itemid;
			Text name;
			Text buyer;
			Text units;
		};

		Texture backgrnd;
		Texture meso_icon;
		std::vector<Record> records;
		Text totals;
		Text total;
		Text received;
		int16_t first_row;
		Slider slider;
	};

	// A hired merchant owner's list of names (0x51a1ec, 0x519781): a plain
	// window centred on the screen whose rows are clicked to select one.
	// OK, Enter and Esc close it and send nothing.
	class UIMerchantList : public UIElement
	{
	public:
		void draw(float inter) const override;

		Cursor::State send_cursor(bool clicked, Point<int16_t> cursorpos) override;

		void remove_cursor() override;

		void send_key(int32_t keycode, bool pressed, bool escape) override;

	protected:
		UIMerchantList(const std::string &background, int16_t detail_x);

		// Where row `index` is, inside the window; the right and bottom
		// edges are outside it.
		virtual Rectangle<int16_t> cell(size_t index) const = 0;

		virtual size_t capacity() const = 0;

		// The buttons that follow the selection and the count.
		virtual void refresh_buttons() = 0;

		bool has_selection() const;

		// Closes the list, giving the shop its button back.
		void close();

		void add_row(const std::string &name, const std::string &detail = "");

		void remove_selected();

		struct Row
		{
			std::string name;
			Text name_text;
			Text detail;
		};

		std::vector<Row> rows;
		int16_t selected;

	private:
		void select(int16_t index);

		Texture backgrnd;
		// Where a row's detail (a visit's time) starts after its name.
		int16_t detail_x;
		bool mouse_down = false;
	};

	// Who visited the store, and how long ago (0x51a1ec).
	class UIMerchantVisitors : public UIMerchantList
	{
	public:
		static constexpr Type TYPE = UIElement::Type::MERCHANTVISITORS;
		static constexpr bool FOCUSED = false;
		static constexpr bool TOGGLED = false;

		UIMerchantVisitors(const std::vector<hired_merchant::Visitor> &visitors);

		UIElement::Type get_type() const override;

	protected:
		Button::State button_pressed(uint16_t buttonid) override;

		Rectangle<int16_t> cell(size_t index) const override;

		size_t capacity() const override;

		void refresh_buttons() override;

	private:
		enum Buttons : uint16_t
		{
			SAVE_NAME,
			OK
		};
	};

	// The names turned away from the store (0x519781). Adding and deleting
	// tell the server at once, which doesn't answer.
	class UIMerchantBlacklist : public UIMerchantList
	{
	public:
		static constexpr Type TYPE = UIElement::Type::MERCHANTBLACKLIST;
		static constexpr bool FOCUSED = false;
		static constexpr bool TOGGLED = false;

		UIMerchantBlacklist(const std::vector<std::string> &names);

		UIElement::Type get_type() const override;

		// Adds a typed name if it may be (0x519bac).
		void add(const std::string &typed);

	protected:
		Button::State button_pressed(uint16_t buttonid) override;

		Rectangle<int16_t> cell(size_t index) const override;

		size_t capacity() const override;

		void refresh_buttons() override;

	private:
		enum Buttons : uint16_t
		{
			ADD,
			REMOVE,
			OK
		};
	};
}
