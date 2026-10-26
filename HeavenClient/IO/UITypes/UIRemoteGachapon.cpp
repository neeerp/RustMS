#include "UIRemoteGachapon.h"

#include "../Components/MapleButton.h"

#include "../../Character/Inventory/RemoteGachapon.h"
#include "../../Constants.h"
#include "../../Net/Packets/GameplayPackets.h"
#include "../../Util/NxFiles.h"

#include <nlnx/nx.hpp>

namespace ms
{
	namespace
	{
		constexpr int16_t DRAG_HEIGHT = 25;

		Point<int16_t> point(remote_gachapon::Place place)
		{
			return Point<int16_t>(place.x, place.y);
		}
	}

	UIRemoteGachapon::UIRemoteGachapon(int32_t id)
			: UIElement(Point<int16_t>(), Point<int16_t>()), item_id(id), dragged(false)
	{
		nl::node src = NxFiles::ui_classic["UIWindow.img"]["RemoteGachapon"];

		Texture background = src["backgrnd"];
		sprites.emplace_back(src["backgrnd"]);

		for (uint16_t town = 0; town < remote_gachapon::TOWNS.size(); town++)
			buttons[town] = std::make_unique<MapleButton>(src["Bt" + std::to_string(town)],
														  point(remote_gachapon::TOWNS[town]));

		buttons[CLOSE] = std::make_unique<MapleButton>(NxFiles::ui_classic["Basic.img"]["BtClose"],
													   point(remote_gachapon::CLOSE));

		// Centred, as v83 centres it in 800x600 (0x4edaef).
		dimension = background.get_dimensions();
		position = Point<int16_t>((Constants::Constants::get().get_viewwidth() - dimension.x()) / 2,
								  (Constants::Constants::get().get_viewheight() - dimension.y()) / 2);
	}

	void UIRemoteGachapon::remove_cursor()
	{
		UIElement::remove_cursor();

		dragged = false;
	}

	Cursor::State UIRemoteGachapon::send_cursor(bool clicked, Point<int16_t> cursorpos)
	{
		if (dragged)
		{
			if (clicked)
			{
				position = cursorpos - drag_from;

				return Cursor::State::CLICKING;
			}

			dragged = false;
		}

		Point<int16_t> cursor = cursorpos - position;
		Cursor::State ret = UIElement::send_cursor(clicked, cursorpos);

		// The top is the caption.
		if (clicked && cursor.y() >= 0 && cursor.y() < DRAG_HEIGHT && !is_holding_button())
		{
			drag_from = cursor;
			dragged = true;
		}

		return ret;
	}

	void UIRemoteGachapon::send_key(int32_t keycode, bool pressed, bool escape)
	{
		// Escape and Enter both close it without sending (0x4edd21).
		if (pressed && (escape || keycode == KeyAction::Id::RETURN))
			deactivate();
	}

	UIElement::Type UIRemoteGachapon::get_type() const
	{
		return TYPE;
	}

	Button::State UIRemoteGachapon::button_pressed(uint16_t buttonid)
	{
		// 0x811a64: a town sends the ticket and its index, then the window closes either way.
		if (buttonid < remote_gachapon::TOWNS.size())
			UseRemoteGachaponPacket(item_id, buttonid).dispatch();

		deactivate();

		return Button::State::NORMAL;
	}
}
