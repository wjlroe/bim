module TestingThings
  class AThing
    def initialize(something)
      @something = something
    end
  end
end

woot = TestingThings::AThing.new('yup')
puts woot.inspect
