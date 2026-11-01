#include "GuildBoss.h"

namespace ms
{
	namespace guild_boss
	{
		namespace
		{
			int32_t entered = 0;
			Rules current_rules;
		}

		bool Rect::intersects(const Rect& other) const
		{
			return left < other.right && other.left < right && top < other.bottom && other.top < bottom;
		}

		Rect Rules::pulley_box() const
		{
			return { pulley_x - 186, pulley_y + 90, pulley_x - 60, pulley_y + 184 };
		}

		void enter_field(int32_t fieldtype, const Rules& rules)
		{
			entered = fieldtype;
			current_rules = rules;
			state() = State{ rules.y_min, 0 };
		}

		bool on_field()
		{
			return entered == FIELD_TYPE;
		}

		const Rules& rules()
		{
			return current_rules;
		}

		State& state()
		{
			static State current;

			return current;
		}

		int16_t read_healer_move(InPacket& recv)
		{
			return recv.read_short();
		}

		uint8_t read_pulley_state(InPacket& recv)
		{
			return static_cast<uint8_t>(recv.read_byte());
		}
	}
}
