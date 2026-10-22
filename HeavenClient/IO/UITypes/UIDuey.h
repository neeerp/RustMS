#pragma once

#include <cstdint>
#include <deque>
#include <vector>

#include "../UIDragElement.h"

#include "../Components/Slider.h"
#include "../Graphics/Geometry.h"
#include "../Graphics/Text.h"

#include "../Net/Handlers/Parcels.h"

namespace ms
{
	// Duey's window (v83 CParcelDlg, report b028): the parcels waiting for
	// the player, and the panels that send them.
	class UIDuey : public UIDragElement<PosDUEY>
	{
	public:
		static constexpr Type TYPE = UIElement::Type::DUEY;
		static constexpr bool FOCUSED = false;
		static constexpr bool TOGGLED = false;

		// What the window holds (the constructor's type).
		enum class Kind : uint8_t
		{
			// The parcels and a normal send: Duey's own.
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

	protected:
		Button::State button_pressed(uint16_t buttonid) override;

	private:
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

		static Row make_row(const parcels::Parcel &parcel);

		void draw_list() const;
		void draw_chosen(const Row &row) const;
		const Row *chosen() const;
		void choose(int16_t index);
		void refresh_slider();
		void show_item(Point<int16_t> cursor);
		void clear_tooltip();
		void claim();
		void remove_chosen();
		void close();

		enum Buttons : uint16_t
		{
			BT_CLOSE,
			BT_RECEIVE,
			BT_DELETE
		};

		Kind kind;
		std::vector<Row> rows;
		int16_t offset;
		int16_t selection;
		// A claim or delete is waiting for its answer.
		bool pending;
		// The notices of parcels that expired, still to be shown.
		std::deque<std::string> expired;

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
	};
}
