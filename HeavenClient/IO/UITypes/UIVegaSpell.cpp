#include "UIVegaSpell.h"

#include "UINotice.h"

#include "../UI.h"

#include "../Components/MapleButton.h"

#include "../../Constants.h"
#include "../../Character/Inventory/CashUpgrades.h"
#include "../../Data/ItemData.h"
#include "../../Gameplay/Stage.h"
#include "../../Net/Packets/InventoryPackets.h"
#include "../../Util/NxFiles.h"

#include <cmath>
#include <map>

#include <nlnx/nx.hpp>

namespace ms
{
	namespace
	{
		// From the window's top left (FUN_0082bb83, FUN_0082c28b). The icons'
		// origin is their bottom left.
		constexpr Point<int16_t> EQUIP_ICON = Point<int16_t>(30, 157);
		constexpr Point<int16_t> SCROLL_ICON = Point<int16_t>(117, 157);
		constexpr Rectangle<int16_t> EQUIP_WELL = Rectangle<int16_t>(29, 61, 125, 157);
		constexpr Rectangle<int16_t> SCROLL_WELL = Rectangle<int16_t>(116, 148, 125, 157);
		// The arrow between the rates is drawn without its origin at
		// (80, 36), and plays at Start over the same place.
		constexpr Point<int16_t> ARROW = Point<int16_t>(91, 41);
		// The middles of the scroll's rate and the spell's.
		constexpr Point<int16_t> RATE = Point<int16_t>(40, 41);
		constexpr Point<int16_t> RAISED = Point<int16_t>(132, 41);
		constexpr Point<int16_t> SPELLING = Point<int16_t>(85, 114);

		// The gauge fills columns 19 to 159 of row 182 at 63 a second
		// (0x82c61a) and is full past 159 px.
		constexpr int16_t GAUGE_LEFT = 19;
		constexpr int16_t GAUGE_RIGHT = 159;
		constexpr int16_t GAUGE_ROW = 182;
		constexpr int32_t GAUGE_SPEED = 63;

		// 358's codes.
		constexpr int8_t UNKNOWN = 0x3f;
		constexpr int8_t STARTED = 0x40;
		constexpr int8_t STARTED_FAILING = 0x45;
		constexpr int8_t SUCCEEDED = 0x41;
		constexpr int8_t FAILED = 0x43;

		// StringPool 0x151a, 0x151b, 0x151c, 0x15c4, 0x10a, 0x42.
		constexpr const char *SCROLL_REFUSED = "This scroll cannot be used.";
		constexpr const char *SOMETHING_WRONG = "Something wrong!";
		constexpr const char *NO_FIT = "The item and the scroll cannot be combined.";
		constexpr const char *ITEM_REFUSED = "This item cannot be used.";
		constexpr const char *BREATH = "You can't do it right now.\\nCatch your breath and then try again.";
		constexpr const char *DEAD = "You can't do it once you're dead.";

		// From the result window's top left (FUN_0082da77, 0x82dcfc).
		constexpr Point<int16_t> RESULT_ICON = Point<int16_t>(60, 99);
		constexpr Point<int16_t> RESULT_SUCCESS = Point<int16_t>(79, 71);
		constexpr Point<int16_t> RESULT_FAIL = Point<int16_t>(6, 45);
		constexpr Rectangle<int16_t> RESULT_HOVER = Rectangle<int16_t>(60, 87, 70, 97);

		void notice(const std::string &text)
		{
			UI::get().emplace<UIOk>(text, [](bool) {});
		}

		Point<int16_t> centred(Point<int16_t> dimension)
		{
			return Point<int16_t>((Constants::Constants::get().get_viewwidth() - dimension.x()) / 2,
								  (Constants::Constants::get().get_viewheight() - dimension.y()) / 2);
		}

		int32_t item_at(InventoryType::Id tab, int16_t slot)
		{
			return slot ? Stage::get().get_player().get_inventory().get_item_id(tab, slot) : 0;
		}
	}

	UIVegaSpell::UIVegaSpell(int16_t s, int32_t id)
			: UIElement(Point<int16_t>(), Point<int16_t>()), slot(s), item_id(id), equip_slot(0), scroll_slot(0),
			  rate(0), state(State::IDLE), elapsed(0), start_code(UNKNOWN), result_code(UNKNOWN),
			  arrow_playing(false), spelling_playing(false)
	{
		nl::node basic = NxFiles::ui_classic["Basic.img"];
		nl::node src = NxFiles::ui_classic["UIWindow.img"]["VegaSpell"];
		nl::node backgrnd = src[id == vega_spell::SIXTY ? "backgrnd60" : "backgrnd10"];

		Texture background = backgrnd;
		sprites.emplace_back(backgrnd);

		gauge = src["GaugeBar"]["gauge"];
		arrow = src["EffectArrow"]["0"];

		for (size_t i = 0; i < digits.size(); i++)
			digits[i] = src["Count"][std::to_string(i)];

		arrow_effect = src["EffectArrow"];
		spelling = src["EffectSpelling"];
		twinkling = src["EffectTwinkling"];
		twinkle = Sound(nl::nx::sound["UI.img"]["VegaTwinkling"]);

		buttons[Buttons::START] = std::make_unique<MapleButton>(src["BtStart"], Point<int16_t>(67, 209));
		buttons[Buttons::CLOSE] = std::make_unique<MapleButton>(basic["BtClose"], Point<int16_t>(158, 8));

		dimension = background.get_dimensions();
		position = centred(dimension);
	}

	UIVegaSpell::~UIVegaSpell()
	{
		twinkle.stop();
	}

	void UIVegaSpell::draw(float alpha) const
	{
		UIElement::draw(alpha);

		if (int32_t equip = item_at(InventoryType::Id::EQUIP, equip_slot))
			ItemData::get(equip).get_icon(false).draw(position + EQUIP_ICON);

		if (int32_t scroll = item_at(InventoryType::Id::USE, scroll_slot))
			ItemData::get(scroll).get_icon(false).draw(position + SCROLL_ICON);

		if (rate)
		{
			draw_rate(rate, RATE);
			draw_rate(vega_spell::raised(item_id, rate), RAISED);
		}

		if (arrow_playing)
			arrow_effect.draw(position + ARROW, alpha);
		else
			arrow.draw(position + ARROW);

		if (state == State::ROLLING || state == State::DONE)
		{
			int16_t right = std::min<int32_t>(GAUGE_LEFT + GAUGE_SPEED * elapsed / 1000, GAUGE_RIGHT);

			for (int16_t x = GAUGE_LEFT; x <= right; x++)
				gauge.draw(position + Point<int16_t>(x, GAUGE_ROW));
		}

		if (spelling_playing)
			spelling.draw(position + SPELLING, alpha);

		if (state == State::ROLLING)
			twinkling.draw(position + SPELLING, alpha);
	}

	void UIVegaSpell::update()
	{
		UIElement::update();

		if (arrow_playing && arrow_effect.update())
			arrow_playing = false;

		if (spelling_playing && spelling.update())
			spelling_playing = false;

		// An empty stat change unlocks the client when the server refuses the
		// spell, and then no start code comes.
		if (state == State::SENT && Stage::get().get_player().can_request(0, true))
		{
			state = State::IDLE;
			buttons[Buttons::START]->set_state(Button::State::NORMAL);
			notice(ITEM_REFUSED);

			return;
		}

		if (state != State::ROLLING)
			return;

		twinkling.update();
		elapsed += Constants::TIMESTEP;

		// The gauge waits full for the result's code.
		if (GAUGE_LEFT + GAUGE_SPEED * elapsed / 1000 > GAUGE_RIGHT && result_code != UNKNOWN)
			finish();
	}

	void UIVegaSpell::remove_cursor()
	{
		UIElement::remove_cursor();

		clear_tooltip();
	}

	Cursor::State UIVegaSpell::send_cursor(bool clicked, Point<int16_t> cursorpos)
	{
		Point<int16_t> cursor = cursorpos - position;

		if (equip_slot && EQUIP_WELL.contains(cursor))
			UI::get().show_equip(Tooltip::Parent::VEGASPELL, equip_slot);
		else if (int32_t scroll = item_at(InventoryType::Id::USE, scroll_slot); scroll && SCROLL_WELL.contains(cursor))
			UI::get().show_item(Tooltip::Parent::VEGASPELL, scroll);
		else
			clear_tooltip();

		return UIElement::send_cursor(clicked, cursorpos);
	}

	void UIVegaSpell::send_key(int32_t, bool pressed, bool escape)
	{
		// The spell and the scroll are gone once it is sent, so the roll
		// must end in its result.
		if (pressed && escape && state == State::IDLE)
			close();
	}

	bool UIVegaSpell::send_icon(const Icon &icon, Point<int16_t>)
	{
		// FUN_0082ce58: until Start.
		if (state != State::IDLE)
			return true;

		auto [tab, s] = icon.get_source();
		int32_t id = tab == InventoryType::Id::NONE ? 0 : item_at(tab, s);

		if (tab == InventoryType::Id::EQUIP && id / 1000000 == 1)
		{
			equip_slot = s;
		}
		else if (tab == InventoryType::Id::USE)
		{
			int32_t listed = vega_spell::takes_scroll(id) ? listed_rate(id) : 0;

			if (!listed)
			{
				notice(SCROLL_REFUSED);

				return true;
			}

			scroll_slot = s;
			rate = listed;
		}
		else
		{
			notice(ITEM_REFUSED);
		}

		return true;
	}

	UIElement::Type UIVegaSpell::get_type() const
	{
		return TYPE;
	}

	void UIVegaSpell::receive(int8_t code)
	{
		switch (code)
		{
			case STARTED:
			case STARTED_FAILING:
				if (state != State::SENT)
					break;

				start_code = code;
				state = State::ROLLING;
				elapsed = 0;
				spelling.play(Animation::End::FIRST);
				spelling_playing = true;
				twinkling.play(Animation::End::LOOP);
				twinkle.play_loop();
				break;
			case SUCCEEDED:
			case FAILED:
				if (state == State::ROLLING)
					result_code = code;

				break;
			default:
				close();
				notice(ITEM_REFUSED);
				break;
		}
	}

	Button::State UIVegaSpell::button_pressed(uint16_t buttonid)
	{
		if (buttonid == Buttons::START)
		{
			start();

			return state == State::IDLE ? Button::State::NORMAL : Button::State::DISABLED;
		}

		if (state == State::IDLE)
			close();

		return Button::State::NORMAL;
	}

	void UIVegaSpell::start()
	{
		// FUN_0082cbe2.
		int32_t equip = item_at(InventoryType::Id::EQUIP, equip_slot);
		int32_t scroll = item_at(InventoryType::Id::USE, scroll_slot);

		if (!equip || !scroll)
		{
			notice(SOMETHING_WRONG);

			return;
		}

		Player &player = Stage::get().get_player();

		if (!player.can_request(ExclRequest::WAIT, true))
		{
			notice(BREATH);

			return;
		}

		if (player.is_dead())
		{
			notice(DEAD);

			return;
		}

		if (!scroll_rules::fits(scroll, equip))
		{
			notice(NO_FIT);

			return;
		}

		// The items may have moved since they were dropped in, and the spell
		// raises only its own rate.
		rate = vega_spell::takes_scroll(scroll) ? listed_rate(scroll) : 0;

		if (!vega_spell::raised(item_id, rate))
		{
			notice(SCROLL_REFUSED);

			return;
		}

		if (!player.request(UseVegaSpellPacket(slot, item_id, equip_slot, scroll_slot), ExclRequest::WAIT))
			return;

		state = State::SENT;
		arrow_effect.play(Animation::End::FIRST);
		arrow_playing = true;
	}

	void UIVegaSpell::finish()
	{
		state = State::DONE;
		twinkle.stop();

		// The start code picks the window (0x82c4f4).
		if (start_code == STARTED || start_code == STARTED_FAILING)
			UI::get().emplace<UIVegaResult>(start_code == STARTED, equip_slot);
		else
			notice(ITEM_REFUSED);

		Stage::get().get_player().request_answered();
		close();
	}

	void UIVegaSpell::close()
	{
		twinkle.stop();
		clear_tooltip();
		deactivate();
	}

	void UIVegaSpell::clear_tooltip()
	{
		UI::get().clear_tooltip(Tooltip::Parent::VEGASPELL);
	}

	int32_t UIVegaSpell::listed_rate(int32_t scroll_id)
	{
		// Etc's VegaSpell.img, whose prob is a fraction, written as the string
		// "[R8]0.1" in ours.
		static const std::map<int32_t, int32_t> rates = []()
		{
			std::map<int32_t, int32_t> found;

			for (nl::node entry : nl::nx::etc["VegaSpell.img"])
			{
				nl::node prob = entry["prob"];
				double fraction = 0;

				// get_real would read the string with stod, which throws on the tag.
				if (prob.data_type() != nl::node::type::string)
				{
					fraction = prob.get_real();
				}
				else
				{
					std::string text = prob.get_string();
					std::string::size_type at = text.find(']');

					try
					{
						fraction = std::stod(at == std::string::npos ? text : text.substr(at + 1));
					}
					catch (const std::exception &)
					{
						fraction = 0;
					}
				}

				found[entry["item"].get_integer()] = static_cast<int32_t>(std::lround(fraction * 100));
			}

			return found;
		}();

		auto iter = rates.find(scroll_id);

		return iter == rates.end() ? 0 : iter->second;
	}

	void UIVegaSpell::draw_rate(int32_t value, Point<int16_t> centre) const
	{
		if (value <= 0)
			return;

		std::string text = std::to_string(value);
		int16_t width = static_cast<int16_t>(text.size() * 12);
		int16_t x = centre.x() - width / 2 + 6;

		for (char c : text)
		{
			digits[c - '0'].draw(position + Point<int16_t>(x, centre.y()));
			x += 12;
		}
	}

	UIVegaResult::UIVegaResult(bool success, int16_t slot)
			: UIElement(Point<int16_t>(), Point<int16_t>()), equip_slot(slot), playing(true)
	{
		nl::node src = NxFiles::ui_classic["UIWindow.img"]["VegaSpell"];
		nl::node backgrnd = src[success ? "SuccessWnd" : "FailWnd"];

		Texture background = backgrnd;
		sprites.emplace_back(backgrnd);

		effect = src[success ? "EffectSuccess" : "EffectFail"];
		effect.play(Animation::End::FIRST);
		effect_at = success ? RESULT_SUCCESS : RESULT_FAIL;

		buttons[0] = std::make_unique<MapleButton>(src["BtOK"], Point<int16_t>(55, 135));

		dimension = background.get_dimensions();
		position = centred(dimension);

		Sound(nl::nx::sound["UI.img"][success ? "VegaSuccess" : "VegaFail"]).play();
	}

	void UIVegaResult::draw(float alpha) const
	{
		UIElement::draw(alpha);

		if (int32_t equip = item_at(InventoryType::Id::EQUIP, equip_slot))
			ItemData::get(equip).get_icon(false).draw(position + RESULT_ICON);

		if (playing)
			effect.draw(position + effect_at, alpha);
	}

	void UIVegaResult::update()
	{
		UIElement::update();

		if (playing && effect.update())
			playing = false;
	}

	void UIVegaResult::remove_cursor()
	{
		UIElement::remove_cursor();

		UI::get().clear_tooltip(Tooltip::Parent::VEGARESULT);
	}

	Cursor::State UIVegaResult::send_cursor(bool clicked, Point<int16_t> cursorpos)
	{
		if (item_at(InventoryType::Id::EQUIP, equip_slot) && RESULT_HOVER.contains(cursorpos - position))
			UI::get().show_equip(Tooltip::Parent::VEGARESULT, equip_slot);
		else
			UI::get().clear_tooltip(Tooltip::Parent::VEGARESULT);

		return UIElement::send_cursor(clicked, cursorpos);
	}

	void UIVegaResult::send_key(int32_t keycode, bool pressed, bool escape)
	{
		if (pressed && (escape || keycode == KeyAction::Id::RETURN))
			close();
	}

	UIElement::Type UIVegaResult::get_type() const
	{
		return TYPE;
	}

	Button::State UIVegaResult::button_pressed(uint16_t)
	{
		close();

		return Button::State::NORMAL;
	}

	void UIVegaResult::close()
	{
		UI::get().clear_tooltip(Tooltip::Parent::VEGARESULT);
		deactivate();
	}
}
