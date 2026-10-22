#pragma once

#include <cstdint>
#include <deque>
#include <vector>

#include "../UIDragElement.h"

#include "../Components/Slider.h"
#include "../Components/Textfield.h"
#include "../Graphics/Geometry.h"
#include "../Graphics/Text.h"

#include "../Character/Inventory/InventoryType.h"
#include "../Net/Handlers/Parcels.h"

namespace ms
{
	// Duey's window (v83 CParcelDlg, reports b028, b030 and b032): the
	// parcels waiting for the player, and the panels that send them.
	class UIDuey : public UIDragElement<PosDUEY>
	{
	public:
		static constexpr Type TYPE = UIElement::Type::DUEY;
		static constexpr bool FOCUSED = false;
		static constexpr bool TOGGLED = false;

		// What the window holds (the constructor's type).
		enum class Kind : uint8_t
		{
			// The parcels and both sends: Duey's own.
			PARCELS,
			// A quick send only, from the Quick Delivery Coupon.
			QUICK_SEND,
			// The parcels only, from the notifier of a quick delivery.
			RECEIVE
		};

		UIDuey(Kind kind);

		void draw(float alpha) const override;

		void update() override;

		void remove_cursor() override;

		Cursor::State send_cursor(bool clicked, Point<int16_t> position) override;

		bool send_icon(const Icon &icon, Point<int16_t> position) override;

		bool is_in_range(Point<int16_t> cursorpos) const override;

		void send_scroll(double yoffset) override;

		void send_key(int32_t keycode, bool pressed, bool escape) override;

		UIElement::Type get_type() const override;

		// The parcels waiting, and those that expired since.
		void open(const parcels::Lists &lists);

		// A parcel was claimed or deleted.
		void removed(int32_t id, bool deleted);

		// A parcel arrived while the window was open.
		void added(const parcels::Parcel &parcel);

		// A request was answered with a result code.
		void answered(uint8_t mode);

		// An item from the inventory, dropped on the window and allowed to
		// go (FUN_006f520e).
		void offer(InventoryType::Id from, int16_t slot, int32_t item_id, int16_t count);

	protected:
		Button::State button_pressed(uint16_t buttonid) override;

	private:
		// The panels, in the order of their tabs.
		enum Tab : uint8_t
		{
			RECEIVE,
			SEND,
			QUICK
		};

		// A parcel in the list, with its name and status in black, and in
		// white for when it's chosen.
		struct Row
		{
			parcels::Parcel parcel;
			Text name[2];
			Text status[2];
			std::string shown;

			void set_status(int64_t now);
		};

		// What a send panel holds (FUN_006f2677, FUN_006f0e91).
		struct Outgoing
		{
			InventoryType::Id tab = InventoryType::Id::NONE;
			int16_t slot = 0;
			int32_t item_id = 0;
			int16_t count = 0;
			int32_t meso = 0;
			// The fee table shows under the panel.
			bool table = true;

			Texture icon;
			Text item_name;
			Text level;
			Text meso_text;
		};

		// One tab of the strip, laid out by its label (FUN_004de15f).
		struct TabBox
		{
			Tab tab;
			int16_t start;
			int16_t end;
		};

		static Row make_row(const parcels::Parcel &parcel);

		void draw_tabs() const;
		void draw_list() const;
		void draw_chosen(const Row &row) const;
		void draw_outgoing(const Outgoing &out) const;
		void copy(const Texture &art, Point<int16_t> at) const;
		std::vector<TabBox> tab_boxes() const;
		bool click_tab(Point<int16_t> cursor);
		void select_tab(Tab next);
		void show_controls();
		const Row *chosen() const;
		void choose(int16_t index);
		void refresh_slider();
		void show_item(Point<int16_t> cursor);
		void clear_tooltip();
		void claim();
		void remove_chosen();
		Outgoing &outgoing(Tab panel);
		const Outgoing &outgoing(Tab panel) const;
		Cursor::State send_panel_cursor(bool clicked, Point<int16_t> cursor);
		void ask_mesos();
		void set_mesos(Tab panel, int32_t meso);
		void put(Tab panel, InventoryType::Id from, int16_t slot, int32_t item_id, int16_t count);
		void take_back(Tab panel);
		void toggle_table();
		void send();
		void send_quick();
		void clear_outgoing();
		void wait(bool waiting);
		void close();

		enum Buttons : uint16_t
		{
			BT_CLOSE,
			BT_RECEIVE,
			BT_DELETE,
			BT_SEND,
			BT_COIN,
			BT_MAX,
			BT_MIN
		};

		Kind kind;
		Tab tab;
		std::vector<Tab> tabs;
		std::vector<Row> rows;
		int16_t offset;
		int16_t selection;
		// A request is waiting for its answer.
		bool pending;
		// The notices of parcels that expired, still to be shown.
		std::deque<std::string> expired;

		// The receive panel.
		Texture receive_title;
		Texture receive_npc;
		Texture bubble;
		Texture empty;
		Texture header;
		Texture row_frame;
		Texture footer;
		Texture table;
		ColorBox selection_box;
		Slider slider;

		Text sender;
		Text meso;
		Text item_name;
		Text level;
		Text message;
		Texture item_icon;

		// The send panels: the normal and the quick.
		Outgoing outgoings[2];
		Texture send_title;
		Texture send_npc;
		Texture send_bubble;
		Texture send_label;
		Texture send_box;
		Texture quick_title;
		Texture quick_npc;
		Texture fees[2];
		Textfield recipient;
		Textfield quick_message;
		// The quick message was clicked, which hides its hint.
		bool touched;
		Text hint[2];

		// The tab strip's pieces (Basic.img/Tab2) and labels.
		Texture tab_left[2];
		Texture tab_middle[3];
		Texture tab_right[2];
		Texture tab_fill[2];
		Texture tab_label[2][3];
	};
}
