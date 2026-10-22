#include "UIItemProtector.h"

#include "UINotice.h"

#include "../UI.h"

#include "../Components/MapleButton.h"

#include "../../Audio/Audio.h"
#include "../../Constants.h"
#include "../../Data/ItemData.h"
#include "../../Gameplay/Stage.h"
#include "../../Net/Packets/InventoryPackets.h"
#include "../../Util/NxFiles.h"

#include <nlnx/nx.hpp>

namespace ms
{
	namespace
	{
		// From the window's top left (CUIItemProtector 0x82a0b6, 0x82a203, 0x82a687, 0x82a589). The icon's origin
		// is its bottom left.
		constexpr Point<int16_t> ICON = Point<int16_t>(74, 127);
		constexpr Rectangle<int16_t> ICON_HOVER = Rectangle<int16_t>(75, 102, 100, 127);
		constexpr int16_t DRAG_HEIGHT = 25;

		// StringPool 0x129.
		constexpr const char *REQUEST_BUSY = "Other requests are in process.\\nPlease try again in a little bit.";

		void notice(const std::string &text)
		{
			UI::get().emplace<UIOk>(text, [](bool) {});
		}
	}

	UIItemProtector::UIItemProtector(int16_t s, int32_t id, int32_t d)
			: UIElement(Point<int16_t>(), Point<int16_t>()), slot(s), item_id(id), days(d),
			  target_tab(InventoryType::Id::NONE), target_slot(0), dragged(false)
	{
		nl::node src = NxFiles::ui_classic["UIWindow.img"]["ItemProtector"];

		Texture background = src["backgrnd"];
		sprites.emplace_back(src["backgrnd"]);

		buttons[Buttons::OK] = std::make_unique<MapleButton>(src["BtOk"], Point<int16_t>(42, 154));
		buttons[Buttons::CANCEL] = std::make_unique<MapleButton>(src["BtCancel"], Point<int16_t>(100, 154));

		// Centred, as v83 centres it in 800x600 (0x4edaef).
		dimension = background.get_dimensions();
		position = Point<int16_t>((Constants::Constants::get().get_viewwidth() - dimension.x()) / 2,
								  (Constants::Constants::get().get_viewheight() - dimension.y()) / 2);
	}

	void UIItemProtector::draw(float alpha) const
	{
		UIElement::draw(alpha);

		// Only the icon: the name shows in the question.
		if (target_tab != InventoryType::Id::NONE)
		{
			int32_t id = Stage::get().get_player().get_inventory().get_item_id(target_tab, target_slot);

			if (id)
				ItemData::get(id).get_icon(false).draw(position + ICON);
		}
	}

	void UIItemProtector::remove_cursor()
	{
		UIElement::remove_cursor();

		dragged = false;
		clear_tooltip();
	}

	Cursor::State UIItemProtector::send_cursor(bool clicked, Point<int16_t> cursorpos)
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
		const Inventory &inventory = Stage::get().get_player().get_inventory();
		int32_t id = target_tab == InventoryType::Id::NONE ? 0 : inventory.get_item_id(target_tab, target_slot);

		// The item in the target's place shows its tooltip (0x82a687).
		if (id && ICON_HOVER.contains(cursor))
		{
			if (target_tab == InventoryType::Id::EQUIP)
				UI::get().show_equip(Tooltip::Parent::ITEMPROTECTOR, target_slot);
			else if (auto item = inventory.get_item(target_tab, target_slot))
				UI::get().show_item(Tooltip::Parent::ITEMPROTECTOR, id, item->get_expiration(), item->get_flags(),
									true, 0, item->get_owner());
		}
		else
		{
			clear_tooltip();
		}

		Cursor::State ret = UIElement::send_cursor(clicked, cursorpos);

		// 0x82a589: the top is the caption.
		if (clicked && cursor.y() >= 0 && cursor.y() < DRAG_HEIGHT && !is_holding_button())
		{
			drag_from = cursor;
			dragged = true;
		}

		return ret;
	}

	void UIItemProtector::send_key(int32_t keycode, bool pressed, bool escape)
	{
		// Escape and Enter both close it without sending (0x4edd21).
		if (pressed && (escape || keycode == KeyAction::Id::RETURN))
			close();
	}

	bool UIItemProtector::send_icon(const Icon &icon, Point<int16_t>)
	{
		// FUN_004f434f: any drop from the Item window, which replaces the last.
		auto [tab, s] = icon.get_source();

		if (tab == InventoryType::Id::NONE || tab == InventoryType::Id::EQUIPPED)
			return true;

		auto target = target_at(tab, s);
		item_guard::Drop drop = target ? item_guard::check(*target) : item_guard::Drop::REFUSED;

		if (drop == item_guard::Drop::REFUSED)
		{
			notice(item_guard::REFUSED);

			return true;
		}

		target_tab = tab;
		target_slot = s;

		if (drop == item_guard::Drop::WARNED)
			notice(item_guard::WARNING);

		return true;
	}

	UIElement::Type UIItemProtector::get_type() const
	{
		return TYPE;
	}

	Button::State UIItemProtector::button_pressed(uint16_t buttonid)
	{
		if (buttonid == Buttons::OK)
			ok();
		else
			close();

		return Button::State::NORMAL;
	}

	std::optional<item_guard::Target> UIItemProtector::target_at(InventoryType::Id tab, int16_t s)
	{
		const Inventory &inventory = Stage::get().get_player().get_inventory();
		int32_t id = inventory.get_item_id(tab, s);

		if (!id)
			return std::nullopt;

		item_guard::Target target;
		target.item_id = id;
		target.equip_tab = tab == InventoryType::Id::EQUIP;
		target.expire_on_logout = ItemData::get(id).get_properties().expire_on_logout;
		target.cash = inventory.get_cash_id(tab, s) != 0;

		if (auto equip = inventory.get_equip(tab, s))
		{
			target.sealed = equip->get_flags() & item_expiry::SEALED;
			target.expiration = equip->get_expiration();
		}
		else if (auto item = inventory.get_item(tab, s))
		{
			target.sealed = item->get_flags() & item_expiry::SEALED;
			target.expiration = item->get_expiration();
		}

		return target;
	}

	void UIItemProtector::ok()
	{
		// 0x82a2ee. The item may have moved since it was dropped in.
		auto target = target_tab == InventoryType::Id::NONE ? std::nullopt : target_at(target_tab, target_slot);

		if (!target)
		{
			notice(item_guard::NO_ITEM);

			return;
		}

		if (item_guard::check(*target) == item_guard::Drop::REFUSED)
		{
			notice(item_guard::REFUSED);

			return;
		}

		std::string name = ItemData::get(target->item_id).get_name();
		std::string question = item_id == item_guard::ITEM_GUARD
			? item_guard::question(name)
			: item_guard::timed_question(name, item_guard::sealed_until(*target, days, item_expiry::now()),
										 target->sealed);

		auto send = [slot = slot, item_id = item_id, tab = target_tab, target_slot = target_slot,
					 named = target->item_id](bool yes)
		{
			if (!yes)
				return;

			Player &player = Stage::get().get_player();

			// The packet names only the places, so neither the guard nor the item asked about may have moved
			// while the question was up.
			auto target = target_at(tab, target_slot);

			if (player.get_inventory().get_item_id(InventoryType::Id::CASH, slot) != item_id || !target ||
				target->item_id != named || item_guard::check(*target) == item_guard::Drop::REFUSED)
			{
				notice(item_guard::NO_ITEM);

				return;
			}

			if (!player.can_request(ExclRequest::WAIT))
			{
				notice(REQUEST_BUSY);

				return;
			}

			if (player.request(UseOnItemPacket(slot, item_id, tab, target_slot), ExclRequest::WAIT))
				Sound(nl::nx::sound["Game.img"]["UseShopItem"]).play();

			if (auto protector = UI::get().get_element<UIItemProtector>())
				protector->close();
		};

		UI::get().emplace<UIYesNo>(question, send);
	}

	void UIItemProtector::close()
	{
		clear_tooltip();
		deactivate();
	}

	void UIItemProtector::clear_tooltip()
	{
		UI::get().clear_tooltip(Tooltip::Parent::ITEMPROTECTOR);
	}
}
