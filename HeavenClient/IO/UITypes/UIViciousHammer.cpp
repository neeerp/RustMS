#include "UIViciousHammer.h"

#include "UINotice.h"

#include "../UI.h"

#include "../Components/MapleButton.h"

#include "../../Constants.h"
#include "../../Character/Inventory/CashUpgrades.h"
#include "../../Data/EquipData.h"
#include "../../Data/ItemData.h"
#include "../../Gameplay/Stage.h"
#include "../../Net/Packets/InventoryPackets.h"
#include "../../Util/NxFiles.h"

namespace ms
{
	namespace
	{
		// From the window's top left (FUN_0082a9bc, FUN_0082ad78, FUN_0082b1f0).
		constexpr Point<int16_t> ICON = Point<int16_t>(40, 122);
		constexpr Rectangle<int16_t> ICON_HOVER = Rectangle<int16_t>(41, 66, 97, 122);
		constexpr Point<int16_t> BAR = Point<int16_t>(6, 152);
		constexpr Point<int16_t> GAUGE = Point<int16_t>(12, 156);
		constexpr int16_t DRAG_HEIGHT = 25;

		// The gauge fills 160 columns at 59 a second (0x82aca3), and USE_HAMMER
		// goes a second after a success shows (0x82b86d).
		constexpr int32_t GAUGE_COLUMNS = 160;
		constexpr int32_t GAUGE_SPEED = 59;
		constexpr int32_t SHOW_SUCCESS = 1000;

		// 354's codes.
		constexpr int32_t APPLIED = 0x39;

		// StringPool 0x129.
		constexpr const char *REQUEST_BUSY = "Other requests are in process.\\nPlease try again in a little bit.";

		void notice(const std::string &text)
		{
			UI::get().emplace<UIOk>(text, [](bool) {});
		}

		int32_t filled(int32_t elapsed)
		{
			return std::min(GAUGE_SPEED * elapsed / 1000, GAUGE_COLUMNS);
		}
	}

	UIViciousHammer::UIViciousHammer(int16_t s)
			: UIElement(Point<int16_t>(), Point<int16_t>()), slot(s), equip_slot(0), state(State::IDLE), elapsed(0),
			  send_at(-1), answered(false), code(0), flag(0), applied(0), dragged(false)
	{
		nl::node src = NxFiles::ui_classic["UIWindow.img"]["ViciousHammer"];

		Texture background = src["backgrnd"];
		sprites.emplace_back(src["backgrnd"]);
		sprites.emplace_back(src["GaugeBar"]["bar"], BAR);

		gauge = src["GaugeBar"]["gauge"];
		pressed = src["EffectP"];
		success = src["EffectE"];

		buttons[Buttons::START] = std::make_unique<MapleButton>(src["BtStart"], Point<int16_t>(42, 179));
		buttons[Buttons::CANCEL] = std::make_unique<MapleButton>(src["BtCancel"], Point<int16_t>(100, 179));

		// Start waits for an equip.
		buttons[Buttons::START]->set_state(Button::State::DISABLED);

		dimension = background.get_dimensions();
		position = Point<int16_t>((Constants::Constants::get().get_viewwidth() - dimension.x()) / 2,
								  (Constants::Constants::get().get_viewheight() - dimension.y()) / 2);
	}

	void UIViciousHammer::draw(float alpha) const
	{
		UIElement::draw(alpha);

		if (equip_slot)
		{
			int32_t id = Stage::get().get_player().get_inventory().get_item_id(InventoryType::Id::EQUIP, equip_slot);

			if (id)
				ItemData::get(id).get_icon(false).draw(position + ICON);
		}

		if (state != State::IDLE)
			for (int16_t i = 0; i < filled(elapsed); i++)
				gauge.draw(position + GAUGE + Point<int16_t>(i, 0));
	}

	void UIViciousHammer::update()
	{
		UIElement::update();

		if (state == State::IDLE || state == State::SHOWN)
			return;

		elapsed += Constants::TIMESTEP;

		if (state == State::FILLING && filled(elapsed) >= GAUGE_COLUMNS)
		{
			state = State::FULL;

			if (answered)
				arm();
		}

		if (state == State::FULL && send_at >= 0 && elapsed >= send_at)
		{
			state = State::SHOWN;
			HammerShownPacket(code, flag).dispatch();
		}
	}

	void UIViciousHammer::remove_cursor()
	{
		UIElement::remove_cursor();

		dragged = false;
		clear_tooltip();
	}

	Cursor::State UIViciousHammer::send_cursor(bool clicked, Point<int16_t> cursorpos)
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

		// The equip shows its tooltip.
		if (equip_slot && ICON_HOVER.contains(cursor))
			UI::get().show_equip(Tooltip::Parent::VICIOUSHAMMER, equip_slot);
		else
			clear_tooltip();

		Cursor::State ret = UIElement::send_cursor(clicked, cursorpos);

		// FUN_0082b1b8: the top is the caption.
		if (clicked && cursor.y() >= 0 && cursor.y() < DRAG_HEIGHT && !is_holding_button())
		{
			drag_from = cursor;
			dragged = true;
		}

		return ret;
	}

	void UIViciousHammer::send_key(int32_t keycode, bool pressed, bool escape)
	{
		// Escape cancels, while Cancel could.
		if (pressed && escape && state == State::IDLE)
			close();
	}

	bool UIViciousHammer::send_icon(const Icon &icon, Point<int16_t>)
	{
		// FUN_004f44cf, then FUN_0082b14e: only while nothing is under way.
		if (state != State::IDLE)
			return true;

		auto [tab, s] = icon.get_source();
		auto refusal = check(tab, s);

		if (refusal != vicious_hammer::Refusal::NONE)
		{
			notice(vicious_hammer::refusal_text(refusal));

			return true;
		}

		equip_slot = s;
		buttons[Buttons::START]->set_state(Button::State::NORMAL);

		return true;
	}

	vicious_hammer::Refusal UIViciousHammer::check(InventoryType::Id tab, int16_t s)
	{
		const Inventory &inventory = Stage::get().get_player().get_inventory();

		int32_t id = tab == InventoryType::Id::NONE ? 0 : inventory.get_item_id(tab, s);
		bool equip_tab = tab == InventoryType::Id::EQUIP;
		int32_t slots = equip_tab ? EquipData::get(id).get_slots() : 0;
		bool cash = equip_tab && inventory.get_cash_id(tab, s) != 0;
		auto equip = equip_tab ? inventory.get_equip(tab, s) : Optional<const Equip>();
		int32_t had = equip ? equip->get_vicious() : 0;

		return vicious_hammer::check(equip_tab, id, slots, cash, had);
	}

	UIElement::Type UIViciousHammer::get_type() const
	{
		return TYPE;
	}

	void UIViciousHammer::answer(int32_t c, int32_t f, int32_t a)
	{
		if (state == State::IDLE)
			return;

		code = c;
		flag = f;
		applied = a;
		answered = true;

		if (state == State::FULL)
			arm();
	}

	void UIViciousHammer::finish(int32_t f)
	{
		close();

		notice(f == 0 ? vicious_hammer::done_text(applied) : vicious_hammer::error_text(f));
	}

	void UIViciousHammer::refuse(int32_t reason)
	{
		close();

		if (reason >= 1 && reason <= 3)
			notice(vicious_hammer::refusal_text(static_cast<vicious_hammer::Refusal>(reason)));
		else
			notice(vicious_hammer::error_text(reason));
	}

	Button::State UIViciousHammer::button_pressed(uint16_t buttonid)
	{
		if (buttonid == Buttons::START)
		{
			start();

			return state == State::IDLE ? Button::State::NORMAL : Button::State::DISABLED;
		}

		close();

		return Button::State::NORMAL;
	}

	void UIViciousHammer::start()
	{
		if (!equip_slot)
		{
			notice(vicious_hammer::refusal_text(vicious_hammer::Refusal::NOT_UPGRADABLE));

			return;
		}

		// The equip may have moved since it was dropped in.
		auto refusal = check(InventoryType::Id::EQUIP, equip_slot);

		if (refusal != vicious_hammer::Refusal::NONE)
		{
			notice(vicious_hammer::refusal_text(refusal));

			return;
		}

		Player &player = Stage::get().get_player();

		if (!player.can_request(ExclRequest::WAIT))
		{
			notice(REQUEST_BUSY);

			return;
		}

		if (!player.request(UseHammerPacket(slot, vicious_hammer::ITEM, equip_slot), ExclRequest::WAIT))
			return;

		state = State::FILLING;
		elapsed = 0;
		buttons[Buttons::CANCEL]->set_state(Button::State::DISABLED);
		player.show_effect(pressed);
	}

	void UIViciousHammer::arm()
	{
		if (code == APPLIED && flag == 0)
		{
			Stage::get().get_player().show_effect(success);
			send_at = elapsed + SHOW_SUCCESS;
		}
		else
		{
			send_at = elapsed;
		}
	}

	void UIViciousHammer::close()
	{
		clear_tooltip();
		deactivate();
	}

	void UIViciousHammer::clear_tooltip()
	{
		UI::get().clear_tooltip(Tooltip::Parent::VICIOUSHAMMER);
	}
}
